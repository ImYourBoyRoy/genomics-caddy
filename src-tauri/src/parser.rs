// ./src-tauri/src/parser.rs
/*
Module Docstring:
Purpose: Validated parser for raw genomic data files (AncestryDNA and 23andMe
formats), including CSV/TSV files and ZIP archives.
Responsibilities:
- Detect vendor format from an explicit header before parsing data rows.
- Preserve no-call values while reporting malformed and duplicate rows.
- Normalize chromosome aliases without silently accepting unknown contigs.
- Produce privacy-safe import diagnostics and provenance metadata.
Coordinate contract:
- Vendor positions are 1-based inclusive and remain 1-based in SnpRecord.
- Liftover converts the input to 0-based half-open chain coordinates and
  converts the mapped result back to 1-based inclusive before database write.
*/

use csv::{ReaderBuilder, Trim};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const MAX_DETECTION_LINES: usize = 200;
const MAX_ZIP_ENTRIES: usize = 32;
const MAX_UNCOMPRESSED_BYTES: u64 = 250 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SnpRecord {
    pub rsid: String,
    pub chromosome: String,
    /// Source-export coordinate: 1-based inclusive. The database stores it in
    /// the assembly-specific column after import normalization.
    pub position: u64,
    pub allele1: String,
    pub allele2: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    AncestryDna,
    TwentyThreeAndMe,
    Unknown,
}

impl FileFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::AncestryDna => "AncestryDNA",
            Self::TwentyThreeAndMe => "23andMe",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ParseDiagnostics {
    pub format: String,
    pub vendor: String,
    pub delimiter: String,
    pub source_build: String,
    pub coordinate_system: String,
    pub allele_orientation: String,
    pub total_rows: usize,
    pub accepted_rows: usize,
    pub malformed_rows: usize,
    pub duplicate_rows: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ImportProvenance {
    pub import_id: String,
    pub source_file_name: String,
    pub source_file_sha256: String,
    pub diagnostics: ParseDiagnostics,
    pub liftover_mapped_rows: usize,
    pub liftover_unmapped_rows: usize,
}

impl ImportProvenance {
    pub fn new(import_id: String, source_file_name: String, source_file_sha256: String, diagnostics: ParseDiagnostics) -> Self {
        Self {
            import_id,
            source_file_name,
            source_file_sha256,
            diagnostics,
            liftover_mapped_rows: 0,
            liftover_unmapped_rows: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedGenome {
    pub records: Vec<SnpRecord>,
    pub diagnostics: ParseDiagnostics,
}

#[derive(Debug, Clone)]
struct DetectedFile {
    format: FileFormat,
    delimiter: u8,
    source_build: String,
    allele_orientation: String,
    has_header: bool,
}

impl DetectedFile {
    fn vendor(&self) -> &'static str {
        match self.format {
            FileFormat::AncestryDna => "AncestryDNA",
            FileFormat::TwentyThreeAndMe => "23andMe",
            FileFormat::Unknown => "Generic genomic export",
        }
    }

    fn delimiter_label(&self) -> String {
        match self.delimiter {
            b'\t' => "TSV".to_string(),
            b',' => "CSV".to_string(),
            _ => "delimited text".to_string(),
        }
    }
}

fn normalize_header_field(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('#')
        .trim_matches('"')
        .to_ascii_lowercase()
        .replace([' ', '_', '-'], "")
}

fn split_probe_line(line: &str) -> Vec<String> {
    let value = line.trim().trim_start_matches('#').trim();
    let delimiter = if value.contains('\t') {
        '\t'
    } else if value.contains(',') {
        ','
    } else {
        ' '
    };
    value
        .split(delimiter)
        .filter(|field| !field.trim().is_empty())
        .map(normalize_header_field)
        .collect()
}

fn delimiter_for_line(line: &str) -> u8 {
    if line.contains('\t') { b'\t' } else { b',' }
}

fn looks_like_ancestry_header(fields: &[String]) -> bool {
    fields.len() >= 5
        && fields.iter().any(|field| field == "rsid" || field == "marker")
        && fields.iter().any(|field| field == "allele1")
        && fields.iter().any(|field| field == "allele2")
        && fields.iter().any(|field| field == "chromosome" || field == "chr")
        && fields.iter().any(|field| field == "position" || field == "pos")
}

fn looks_like_23andme_header(fields: &[String]) -> bool {
    fields.len() >= 4
        && fields.iter().any(|field| field == "rsid" || field == "marker")
        && fields.iter().any(|field| field == "genotype" || field == "call")
        && fields.iter().any(|field| field == "chromosome" || field == "chr")
        && fields.iter().any(|field| field == "position" || field == "pos")
}

fn infer_build_and_orientation(text: &str, format: FileFormat) -> (String, String) {
    let lower = text.to_ascii_lowercase();
    let build = if lower.contains("grch38") || lower.contains("hg38") || lower.contains("build 38") {
        "GRCh38".to_string()
    } else if lower.contains("grch37") || lower.contains("hg19") || lower.contains("build 37") {
        "GRCh37".to_string()
    } else if matches!(format, FileFormat::AncestryDna | FileFormat::TwentyThreeAndMe) {
        "GRCh37 (vendor default; header did not state build)".to_string()
    } else {
        "Unknown".to_string()
    };

    let orientation = if lower.contains("reverse strand") || lower.contains("minus strand") {
        "reverse-strand (vendor header)".to_string()
    } else if lower.contains("forward strand") || lower.contains("plus strand") {
        "forward-strand (vendor header)".to_string()
    } else {
        "Unknown (vendor strand not stated)".to_string()
    };
    (build, orientation)
}

fn coordinate_system_for_build(source_build: &str) -> String {
    if source_build.to_ascii_uppercase().contains("GRCH38") {
        "1-based inclusive GRCh38 source coordinates; GRCh37 is retained when inverse liftover maps".to_string()
    } else {
        "1-based inclusive input; GRCh37 database coordinates".to_string()
    }
}

fn detect_file_info_from_reader<R: BufRead>(mut reader: R) -> Result<DetectedFile, String> {
    let mut line = String::new();
    let mut metadata_text = String::new();
    let mut first_data_shape: Option<(usize, u8)> = None;

    for _ in 0..MAX_DETECTION_LINES {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                metadata_text.push_str(trimmed);
                metadata_text.push('\n');

                let fields = split_probe_line(trimmed);
                if fields.is_empty() {
                    continue;
                }
                let delimiter = delimiter_for_line(trimmed);
                if looks_like_ancestry_header(&fields) {
                    let (source_build, allele_orientation) =
                        infer_build_and_orientation(&metadata_text, FileFormat::AncestryDna);
                    return Ok(DetectedFile {
                        format: FileFormat::AncestryDna,
                        delimiter,
                        source_build,
                        allele_orientation,
                        has_header: true,
                    });
                }
                if looks_like_23andme_header(&fields) {
                    let (source_build, allele_orientation) =
                        infer_build_and_orientation(&metadata_text, FileFormat::TwentyThreeAndMe);
                    return Ok(DetectedFile {
                        format: FileFormat::TwentyThreeAndMe,
                        delimiter,
                        source_build,
                        allele_orientation,
                        has_header: true,
                    });
                }
                if !trimmed.starts_with('#') && first_data_shape.is_none() {
                    first_data_shape = Some((fields.len(), delimiter));
                }
            }
            Err(e) => return Err(format!("Error reading header line: {e}")),
        }
    }

    let (field_count, delimiter) = first_data_shape.unwrap_or((0, b'\t'));
    let format = if field_count >= 5 {
        FileFormat::AncestryDna
    } else if field_count == 4 {
        FileFormat::TwentyThreeAndMe
    } else {
        FileFormat::Unknown
    };
    let (source_build, allele_orientation) = infer_build_and_orientation(&metadata_text, format);
    Ok(DetectedFile {
        format,
        delimiter,
        source_build,
        allele_orientation,
        has_header: false,
    })
}

/// Detects the file format from a buffered reader by inspecting an explicit
/// vendor header, with a conservative headerless fallback for 4/5-column data.
pub fn detect_format_from_reader<R: BufRead>(reader: R) -> Result<FileFormat, String> {
    Ok(detect_file_info_from_reader(reader)?.format)
}

fn normalize_chromosome(chr: &str) -> Option<String> {
    let mut clean = chr.trim().to_ascii_uppercase();
    if let Some(stripped) = clean.strip_prefix("CHR") {
        clean = stripped.to_string();
    }
    let normalized = match clean.as_str() {
        "23" => "X",
        "24" => "Y",
        // Ancestry's legacy numeric export uses 25 for the
        // pseudoautosomal/X-coordinate group and 26 for mitochondrial data.
        // PAR positions in that export are reported against the X reference
        // coordinates, so retaining them as X keeps liftover and coordinate
        // matching on the standard reference chromosome.
        "25" => "X",
        "26" => "MT",
        "M" => "MT",
        value if (1..=22).any(|number| value == number.to_string()) => value,
        "X" | "Y" | "MT" => clean.as_str(),
        _ => return None,
    };
    Some(normalized.to_string())
}

fn normalize_allele(value: &str) -> Option<String> {
    let clean = value.trim().to_ascii_uppercase();
    if clean.is_empty() {
        return Some("-".to_string());
    }
    if clean.len() > 4
        || !clean
            .chars()
            .all(|c| matches!(c, 'A' | 'C' | 'G' | 'T' | 'N' | 'I' | 'D' | '?' | '-' | '0'))
    {
        return None;
    }
    Some(clean)
}

/// Splits a 23andMe genotype string (e.g. AA, AG, -, --, or A) into two
/// allele fields while preserving no-call state.
fn parse_twenty_three_genotype(genotype: &str) -> Option<(String, String)> {
    let clean = genotype.trim().to_ascii_uppercase();
    if clean.is_empty() || clean == "-" || clean == "--" || clean == "00" || clean == "??" {
        return Some(("-".to_string(), "-".to_string()));
    }
    let chars: Vec<char> = clean.chars().collect();
    if chars.len() == 1 {
        return normalize_allele(&chars[0].to_string()).map(|allele| (allele, "-".to_string()));
    }
    if chars.len() == 2 {
        let first = normalize_allele(&chars[0].to_string())?;
        let second = normalize_allele(&chars[1].to_string())?;
        return Some((first, second));
    }
    None
}

fn field<'a>(fields: &'a [&'a str], index: usize) -> &'a str {
    fields.get(index).copied().unwrap_or("")
}

fn parse_record(fields: &[&str], format: FileFormat) -> Result<SnpRecord, String> {
    let (rsid, chromosome, position_text, allele1, allele2) = match format {
        FileFormat::AncestryDna => {
            if fields.len() != 5 {
                return Err("expected exactly five columns".to_string());
            }
            (
                field(fields, 0),
                field(fields, 1),
                field(fields, 2),
                field(fields, 3),
                field(fields, 4),
            )
        }
        FileFormat::TwentyThreeAndMe => {
            if fields.len() != 4 {
                return Err("expected exactly four columns".to_string());
            }
            let alleles = parse_twenty_three_genotype(field(fields, 3))
                .ok_or_else(|| "invalid genotype field".to_string())?;
            // Store owned strings outside this match; these temporary values
            // are copied into the final record below.
            let (allele1, allele2) = alleles;
            let record = build_record(field(fields, 0), field(fields, 1), field(fields, 2), &allele1, &allele2)?;
            return Ok(record);
        }
        FileFormat::Unknown => {
            if fields.len() >= 5 {
                (field(fields, 0), field(fields, 1), field(fields, 2), field(fields, 3), field(fields, 4))
            } else if fields.len() >= 4 {
                let alleles = parse_twenty_three_genotype(field(fields, 3))
                    .ok_or_else(|| "invalid genotype field".to_string())?;
                let (allele1, allele2) = alleles;
                return build_record(field(fields, 0), field(fields, 1), field(fields, 2), &allele1, &allele2);
            } else {
                return Err("expected at least four columns".to_string());
            }
        }
    };
    build_record(rsid, chromosome, position_text, allele1, allele2)
}

fn build_record(
    rsid: &str,
    chromosome: &str,
    position_text: &str,
    allele1: &str,
    allele2: &str,
) -> Result<SnpRecord, String> {
    let rsid = rsid.trim().to_string();
    if rsid.is_empty() {
        return Err("missing marker identifier".to_string());
    }
    let chromosome = normalize_chromosome(chromosome)
        .ok_or_else(|| "unknown chromosome alias".to_string())?;
    let position = position_text
        .trim()
        .parse::<u64>()
        .map_err(|_| "position is not an unsigned integer".to_string())?;
    if position == 0 {
        return Err("position must be a positive 1-based coordinate".to_string());
    }
    let allele1 = normalize_allele(allele1).ok_or_else(|| "invalid allele1 field".to_string())?;
    let allele2 = normalize_allele(allele2).ok_or_else(|| "invalid allele2 field".to_string())?;
    Ok(SnpRecord { rsid, chromosome, position, allele1, allele2 })
}

/// Parses a reader with validated row diagnostics. Genotype values are never
/// included in diagnostics or error messages.
fn parse_reader_with_metadata<R: BufRead>(reader: R, detected: &DetectedFile) -> Result<ParsedGenome, String> {
    let mut csv_reader = ReaderBuilder::new()
        .delimiter(detected.delimiter)
        .has_headers(false)
        .flexible(true)
        .trim(Trim::None)
        .comment(Some(b'#'))
        .from_reader(reader);
    let mut records = Vec::with_capacity(700_000);
    let mut seen: HashMap<String, SnpRecord> = HashMap::new();
    let mut total_rows = 0usize;
    let mut malformed_rows = 0usize;
    let mut duplicate_rows = 0usize;
    let mut warnings = Vec::new();

    for result in csv_reader.records() {
        let record = result.map_err(|e| format!("Error reading delimited row: {e}"))?;
        let fields: Vec<&str> = record.iter().collect();
        if fields.iter().all(|field| field.trim().is_empty()) {
            continue;
        }
        if detected.has_header {
            let normalized_fields: Vec<String> = fields.iter().map(|field| normalize_header_field(field)).collect();
            if looks_like_ancestry_header(&normalized_fields) || looks_like_23andme_header(&normalized_fields) {
                continue;
            }
        }
        total_rows += 1;
        let record = match parse_record(&fields, detected.format) {
            Ok(record) => record,
            Err(error) => {
                malformed_rows += 1;
                if warnings.len() < 3 {
                    warnings.push(format!("Malformed row skipped: {error}"));
                }
                continue;
            }
        };
        let key = record.rsid.to_ascii_lowercase();
        if let Some(previous) = seen.get(&key) {
            duplicate_rows += 1;
            if previous != &record {
                return Err(format!("Conflicting duplicate marker rows detected for {key}; import was not written."));
            }
            continue;
        }
        seen.insert(key, record.clone());
        records.push(record);
    }

    if records.is_empty() {
        return Err(format!("The selected DNA file did not contain any valid genotype records (rows checked: {total_rows}, malformed: {malformed_rows})."));
    }
    if malformed_rows > 0 {
        warnings.push(format!("{malformed_rows} malformed row(s) were skipped."));
    }
    if duplicate_rows > 0 {
        warnings.push(format!("{duplicate_rows} duplicate row(s) were ignored after validation."));
    }

    Ok(ParsedGenome {
        diagnostics: ParseDiagnostics {
            format: detected.format.label().to_string(),
            vendor: detected.vendor().to_string(),
            delimiter: detected.delimiter_label(),
            source_build: detected.source_build.clone(),
            coordinate_system: coordinate_system_for_build(&detected.source_build),
            allele_orientation: detected.allele_orientation.clone(),
            total_rows,
            accepted_rows: records.len(),
            malformed_rows,
            duplicate_rows,
            warnings,
        },
        records,
    })
}

/// Backward-compatible parser entry point returning only validated records.
pub fn parse_reader<R: BufRead>(reader: R, format: FileFormat) -> Result<Vec<SnpRecord>, String> {
    let detected = DetectedFile {
        format,
        delimiter: b'\t',
        source_build: "Unknown".to_string(),
        allele_orientation: "Unknown".to_string(),
        has_header: true,
    };
    Ok(parse_reader_with_metadata(reader, &detected)?.records)
}

/// Parses a ZIP by selecting the best recognized genomic text entry rather
/// than blindly taking the first `.txt` file.
pub fn parse_zip_file_with_metadata<P: AsRef<Path>>(
    path: P,
    progress_callback: &dyn Fn(&str),
) -> Result<ParsedGenome, String> {
    progress_callback("Unpacking ZIP archive...");
    let file = File::open(path).map_err(|e| format!("Failed to open ZIP file: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {e}"))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(format!("ZIP archive exceeds maximum of {MAX_ZIP_ENTRIES} entries"));
    }

    let mut total_size = 0u64;
    let mut best: Option<(usize, i32)> = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("ZIP entry error: {e}"))?;
        total_size = total_size.checked_add(entry.size()).ok_or_else(|| "ZIP size overflow".to_string())?;
        if total_size > MAX_UNCOMPRESSED_BYTES {
            return Err(format!("ZIP archive exceeds maximum uncompressed size of {MAX_UNCOMPRESSED_BYTES} bytes"));
        }
        if entry.is_dir() || entry.size() > MAX_UNCOMPRESSED_BYTES {
            continue;
        }
        let name = entry
            .name()
            .map_err(|e| format!("ZIP entry name error: {e}"))?
            .to_ascii_lowercase();
        if !(name.ends_with(".txt") || name.ends_with(".csv") || name.ends_with(".tsv")) {
            continue;
        }
        let detected = detect_file_info_from_reader(BufReader::new(&mut entry))?;
        if detected.format == FileFormat::Unknown {
            continue;
        }
        let mut score = if detected.has_header { 100 } else { 10 };
        if name.contains("genome") || name.contains("dna") || name.contains("raw") {
            score += 5;
        }
        if best.as_ref().map(|(_, current)| score > *current).unwrap_or(true) {
            best = Some((i, score));
        }
    }
    let index = best.map(|(index, _)| index).ok_or_else(|| "No recognized genomic CSV/TSV/TXT file found inside the ZIP archive".to_string())?;

    let detected = {
        let mut entry_header = archive
            .by_index(index)
            .map_err(|e| format!("Failed to read ZIP entry: {e}"))?;
        detect_file_info_from_reader(BufReader::new(&mut entry_header))?
    };
    progress_callback("Parsing genetic records...");
    let entry_data = archive.by_index(index).map_err(|e| format!("Failed to read ZIP entry: {e}"))?;
    parse_reader_with_metadata(BufReader::new(entry_data), &detected)
}

/// Backward-compatible ZIP parser entry point returning only validated records.
pub fn parse_zip_file<P: AsRef<Path>>(
    path: P,
    progress_callback: &dyn Fn(&str),
) -> Result<Vec<SnpRecord>, String> {
    Ok(parse_zip_file_with_metadata(path, progress_callback)?.records)
}

/// Main parser entry point with diagnostics and provenance inputs.
pub fn parse_dna_file_with_metadata<P: AsRef<Path>, F: Fn(&str)>(
    path: P,
    progress_callback: F,
) -> Result<ParsedGenome, String> {
    let path_ref = path.as_ref();
    let is_zip = path_ref.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
    if is_zip {
        return parse_zip_file_with_metadata(path_ref, &progress_callback);
    }

    progress_callback("Reading file...");
    let file = File::open(path_ref).map_err(|e| format!("Failed to open file: {e}"))?;
    let detected = detect_file_info_from_reader(BufReader::new(file))?;
    progress_callback("Parsing genetic records...");
    let file_parse = File::open(path_ref).map_err(|e| format!("Failed to open file: {e}"))?;
    parse_reader_with_metadata(BufReader::new(file_parse), &detected)
}

/// Backward-compatible parser entry point returning only records.
pub fn parse_dna_file<P: AsRef<Path>, F: Fn(&str)>(path: P, progress_callback: F) -> Result<Vec<SnpRecord>, String> {
    Ok(parse_dna_file_with_metadata(path, progress_callback)?.records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    #[test]
    fn detects_ancestry_header_before_five_column_fallback() {
        let input = b"rsid\tchromosome\tposition\tallele1\tallele2\nrs123\t1\t101\tA\tG\n";
        assert_eq!(detect_format_from_reader(Cursor::new(input)).unwrap(), FileFormat::AncestryDna);
    }

    #[test]
    fn detects_23andme_grch38_metadata_and_records_source_coordinate_contract() {
        let input = b"# This export uses genome build 38\n# rsid\tchromosome\tposition\tgenotype\nrs123\t1\t710\tAG\n";
        let file = detect_file_info_from_reader(Cursor::new(input)).unwrap();
        let parsed = parse_reader_with_metadata(Cursor::new(input), &file).unwrap();
        assert_eq!(parsed.diagnostics.vendor, "23andMe");
        assert_eq!(parsed.diagnostics.source_build, "GRCh38");
        assert!(parsed.diagnostics.coordinate_system.contains("GRCh38 source"));
        assert_eq!(parsed.records[0].position, 710);
    }

    #[test]
    fn parses_ancestry_csv_with_explicit_build_and_orientation_headers() {
        let input = b"# build 37; forward strand\nrsid,chromosome,position,allele1,allele2\nrs123,1,101,A,G\n";
        let file = detect_file_info_from_reader(Cursor::new(input)).unwrap();
        let parsed = parse_reader_with_metadata(Cursor::new(input), &file).unwrap();
        assert_eq!(parsed.diagnostics.vendor, "AncestryDNA");
        assert_eq!(parsed.diagnostics.delimiter, "CSV");
        assert_eq!(parsed.diagnostics.allele_orientation, "forward-strand (vendor header)");
        assert_eq!(parsed.records.len(), 1);
    }

    #[test]
    fn parses_representative_synthetic_vendor_fixture_files() {
        let fixtures = [
            include_str!("../testdata/synthetic_23andme_grch38.txt"),
            include_str!("../testdata/synthetic_ancestry_grch37.csv"),
            include_str!("../testdata/synthetic_23andme_grch38.csv"),
            include_str!("../testdata/synthetic_ancestry_grch37.tsv"),
        ];
        let parsed = fixtures
            .iter()
            .map(|fixture| {
                let file = detect_file_info_from_reader(Cursor::new(fixture.as_bytes())).unwrap();
                parse_reader_with_metadata(Cursor::new(fixture.as_bytes()), &file).unwrap()
            })
            .collect::<Vec<_>>();

        assert_eq!(parsed[0].diagnostics.vendor, "23andMe");
        assert_eq!(parsed[0].diagnostics.source_build, "GRCh38");
        assert_eq!(parsed[0].records.len(), 2);
        assert_eq!(parsed[1].diagnostics.vendor, "AncestryDNA");
        assert_eq!(parsed[1].diagnostics.source_build, "GRCh37");
        assert_eq!(parsed[1].records.len(), 2);
        assert_eq!(parsed[2].diagnostics.vendor, "23andMe");
        assert_eq!(parsed[2].diagnostics.delimiter, "CSV");
        assert_eq!(parsed[2].records.len(), 2);
        assert_eq!(parsed[3].diagnostics.vendor, "AncestryDNA");
        assert_eq!(parsed[3].diagnostics.delimiter, "TSV");
        assert_eq!(parsed[3].records.len(), 2);
    }

    #[test]
    fn parses_csv_and_reports_malformed_and_duplicate_rows_without_genotype_logging() {
        let input = b"# build 37\nrsid,chromosome,position,genotype\nrs123,chr1,101,AG\nrs123,chr1,101,AG\nnot-a-row,chr?,bad,ZZ\n";
        let file = detect_file_info_from_reader(Cursor::new(input)).unwrap();
        let parsed = parse_reader_with_metadata(Cursor::new(input), &file).unwrap();
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.diagnostics.duplicate_rows, 1);
        assert_eq!(parsed.diagnostics.malformed_rows, 1);
        assert!(parsed.diagnostics.warnings.iter().all(|warning| !warning.contains("AG")));
    }

    #[test]
    fn preserves_one_based_position_and_no_call_state() {
        let input = b"rsid\tchromosome\tposition\tgenotype\nrs123\t23\t1\t--\n";
        let file = detect_file_info_from_reader(Cursor::new(input)).unwrap();
        let parsed = parse_reader_with_metadata(Cursor::new(input), &file).unwrap();
        assert_eq!(parsed.records[0].position, 1);
        assert_eq!(parsed.records[0].chromosome, "X");
        assert_eq!(parsed.records[0].allele1, "-");
        assert_eq!(parsed.records[0].allele2, "-");
    }

    #[test]
    fn maps_ancestry_numeric_par_and_mitochondrial_aliases() {
        let input = b"rsid\tchromosome\tposition\tallele1\tallele2\nrspar\t25\t101\tA\tG\nrsmt\t26\t102\tC\tT\n";
        let file = detect_file_info_from_reader(Cursor::new(input)).unwrap();
        let parsed = parse_reader_with_metadata(Cursor::new(input), &file).unwrap();
        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0].chromosome, "X");
        assert_eq!(parsed.records[1].chromosome, "MT");
        assert_eq!(parsed.diagnostics.malformed_rows, 0);
    }

    #[test]
    fn rejects_conflicting_duplicate_marker_rows() {
        let input = b"rsid\tchromosome\tposition\tgenotype\nrs123\t1\t101\tAG\nrs123\t1\t102\tAA\n";
        let file = detect_file_info_from_reader(Cursor::new(input)).unwrap();
        let error = parse_reader_with_metadata(Cursor::new(input), &file).unwrap_err();
        assert!(error.contains("Conflicting duplicate marker rows"));
    }

    #[test]
    fn selects_a_recognized_genome_entry_instead_of_the_first_text_entry() {
        let path = std::env::temp_dir().join(format!(
            "genomics_caddy_zip_parser_test_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let file = File::create(&path).expect("create ZIP fixture");
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("README.txt", SimpleFileOptions::default())
            .expect("start readme entry");
        archive.write_all(b"not a genome export").expect("write readme entry");
        archive
            .start_file("raw_genome.csv", SimpleFileOptions::default())
            .expect("start genome entry");
        archive
            .write_all(b"rsid,chromosome,position,genotype\nrs123,1,101,AG\n")
            .expect("write genome entry");
        archive.finish().expect("finish ZIP fixture");

        let parsed = parse_zip_file_with_metadata(&path, &|_| {}).expect("parse ZIP fixture");
        assert_eq!(parsed.diagnostics.vendor, "23andMe");
        assert_eq!(parsed.records.len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
