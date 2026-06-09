// ./src-tauri/src/parser.rs
/*
Module Docstring:
Purpose: High-performance parser for raw genomic data files (AncestryDNA and 23andMe formats) supporting ZIP archives.
Responsibilities:
- Parse raw TSV/CSV files or ZIP archives containing text exports.
- Standardize chromosome labels (e.g. 1-22, X, Y, MT).
- Normalize genotypes (e.g., split 23andMe double-character genotypes into allele1/allele2).
Key Inputs: Path to raw DNA text or ZIP file.
Key Outputs: Vector of SnpRecord structs.
Operational Notes: Decompresses ZIP archives in-memory without creating temp files.
*/

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnpRecord {
    pub rsid: String,
    pub chromosome: String,
    pub position: u64,
    pub allele1: String,
    pub allele2: String,
}

#[derive(Debug, Clone, Copy)]
pub enum FileFormat {
    AncestryDna,
    TwentyThreeAndMe,
    Unknown,
}

/// Detects the file format from a buffered reader by inspecting the first few non-comment lines.
pub fn detect_format_from_reader<R: BufRead>(mut reader: R) -> Result<FileFormat, String> {
    let mut line = String::new();
    for _ in 0..100 {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                let lower = trimmed.to_lowercase();
                if lower.contains("rsid") && lower.contains("genotype") {
                    return Ok(FileFormat::TwentyThreeAndMe);
                } else if lower.contains("rsid") && lower.contains("allele1") {
                    return Ok(FileFormat::AncestryDna);
                }

                let cols: Vec<&str> = trimmed.split('\t').collect();
                if cols.len() >= 4 {
                    return Ok(FileFormat::TwentyThreeAndMe);
                } else if cols.len() >= 5 {
                    return Ok(FileFormat::AncestryDna);
                }
            }
            Err(e) => return Err(format!("Error reading header line: {}", e)),
        }
    }
    Ok(FileFormat::Unknown)
}

/// Parses standard SNP records from a reader.
pub fn parse_reader<R: BufRead>(reader: R, format: FileFormat) -> Result<Vec<SnpRecord>, String> {
    let mut records = Vec::with_capacity(700_000);

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| format!("Error reading line: {}", e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip headers
        let lower = trimmed.to_lowercase();
        if lower.contains("rsid") {
            continue;
        }

        let cols: Vec<&str> = trimmed.split('\t').collect();
        if cols.is_empty() {
            continue;
        }

        match format {
            FileFormat::AncestryDna => {
                if cols.len() >= 5 {
                    let rsid = cols[0].trim().to_string();
                    let chromosome = normalize_chromosome(cols[1].trim());
                    let position = cols[2].trim().parse::<u64>().unwrap_or(0);
                    let allele1 = cols[3].trim().to_string();
                    let allele2 = cols[4].trim().to_string();

                    if position > 0 && !rsid.is_empty() {
                        records.push(SnpRecord {
                            rsid,
                            chromosome,
                            position,
                            allele1,
                            allele2,
                        });
                    }
                }
            }
            FileFormat::TwentyThreeAndMe => {
                if cols.len() >= 4 {
                    let rsid = cols[0].trim().to_string();
                    let chromosome = normalize_chromosome(cols[1].trim());
                    let position = cols[2].trim().parse::<u64>().unwrap_or(0);
                    let genotype = cols[3].trim();

                    let (allele1, allele2) = parse_twenty_three_genotype(genotype);

                    if position > 0 && !rsid.is_empty() {
                        records.push(SnpRecord {
                            rsid,
                            chromosome,
                            position,
                            allele1,
                            allele2,
                        });
                    }
                }
            }
            FileFormat::Unknown => {
                if cols.len() >= 5 {
                    let rsid = cols[0].trim().to_string();
                    let chromosome = normalize_chromosome(cols[1].trim());
                    let position = cols[2].trim().parse::<u64>().unwrap_or(0);
                    let allele1 = cols[3].trim().to_string();
                    let allele2 = cols[4].trim().to_string();
                    if position > 0 {
                        records.push(SnpRecord { rsid, chromosome, position, allele1, allele2 });
                    }
                } else if cols.len() >= 4 {
                    let rsid = cols[0].trim().to_string();
                    let chromosome = normalize_chromosome(cols[1].trim());
                    let position = cols[2].trim().parse::<u64>().unwrap_or(0);
                    let (allele1, allele2) = parse_twenty_three_genotype(cols[3].trim());
                    if position > 0 {
                        records.push(SnpRecord { rsid, chromosome, position, allele1, allele2 });
                    }
                }
            }
        }
    }

    Ok(records)
}

/// Helper callback for reporting unzip progress.
pub fn parse_zip_file<P: AsRef<Path>>(
    path: P,
    progress_callback: &dyn Fn(&str),
) -> Result<Vec<SnpRecord>, String> {
    progress_callback("Unpacking ZIP archive...");
    let file = File::open(path).map_err(|e| format!("Failed to open ZIP file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {}", e))?;

    let mut found_index = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("ZIP entry error: {}", e))?;
        let entry_name = entry.name().map_err(|e| format!("ZIP entry name error: {}", e))?;
        if entry_name.to_lowercase().ends_with(".txt") {
            found_index = Some(i);
            break;
        }
    }

    let idx = found_index.ok_or_else(|| "No .txt file found inside the ZIP archive".to_string())?;

    // Open first time to detect format
    let entry_header = archive.by_index(idx).map_err(|e| format!("Failed to read ZIP entry: {}", e))?;
    let format = detect_format_from_reader(BufReader::new(entry_header))?;

    // Open second time to parse
    progress_callback("Parsing genetic records...");
    let entry_data = archive.by_index(idx).map_err(|e| format!("Failed to read ZIP entry: {}", e))?;
    let reader = BufReader::new(entry_data);
    parse_reader(reader, format)
}

/// Main entry point to parse a file path, automatically detecting if it is a ZIP or TXT.
pub fn parse_dna_file<P: AsRef<Path>, F: Fn(&str)>(
    path: P,
    progress_callback: F,
) -> Result<Vec<SnpRecord>, String> {
    let path_ref = path.as_ref();
    let is_zip = path_ref.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("zip"));

    if is_zip {
        parse_zip_file(path_ref, &progress_callback)
    } else {
        progress_callback("Reading file...");
        let file = File::open(path_ref).map_err(|e| format!("Failed to open file: {}", e))?;
        
        // Detect format
        let format = detect_format_from_reader(BufReader::new(&file))?;
        
        // Re-open to parse from start
        let file_parse = File::open(path_ref).map_err(|e| format!("Failed to open file: {}", e))?;
        progress_callback("Parsing genetic records...");
        parse_reader(BufReader::new(file_parse), format)
    }
}

/// Normalizes chromosome labels to strings like "1".."22", "X", "Y", "MT".
fn normalize_chromosome(chr: &str) -> String {
    let clean = chr.replace("chr", "").replace("Chr", "").trim().to_string();
    match clean.as_str() {
        "23" => "X".to_string(),
        "24" => "Y".to_string(),
        "25" => "MT".to_string(),
        _ => clean,
    }
}

/// Splits 23andMe genotype string (e.g. "AA", "AG", "-", "A") into two alleles.
fn parse_twenty_three_genotype(genotype: &str) -> (String, String) {
    let chars: Vec<char> = genotype.chars().collect();
    if chars.is_empty() {
        ("-".to_string(), "-".to_string())
    } else if chars.len() == 1 {
        (chars[0].to_string(), "-".to_string())
    } else if chars.len() == 2 {
        (chars[0].to_string(), chars[1].to_string())
    } else {
        ("-".to_string(), "-".to_string())
    }
}
