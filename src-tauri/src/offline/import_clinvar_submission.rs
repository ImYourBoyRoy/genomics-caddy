use crate::offline::compress::open_text_auto;
use csv::{ReaderBuilder, StringRecord};
use rusqlite::{Connection, params};
use std::time::Instant;
use std::{
    io::{BufRead, Cursor, Read},
    path::Path,
};
use tauri::Emitter;

const MAX_HEADER_SCAN_LINES: usize = 256;
const MAX_HEADER_SCAN_BYTES: usize = 1024 * 1024;

fn normalize_header(value: &str) -> String {
    value
        .trim_start_matches('\u{feff}')
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn find_column(headers: &StringRecord, names: &[&str]) -> Option<usize> {
    headers.iter().position(|header| {
        let normalized = normalize_header(header);
        names
            .iter()
            .any(|name| normalized == normalize_header(name))
    })
}

fn is_submission_header_candidate(line: &str) -> bool {
    let candidate = line.trim_start_matches('#').trim_end_matches(['\r', '\n']);
    let mut has_variation_id = false;
    let mut has_scv = false;
    for field in candidate.split('\t') {
        match normalize_header(field).as_str() {
            "variationid" => has_variation_id = true,
            "scv" => has_scv = true,
            _ => {}
        }
    }
    has_variation_id && has_scv
}

/// NCBI's submission summary starts with descriptive `##` records, then a
/// tab-delimited header whose first field is also prefixed with `#`. Find that
/// header without buffering the large data file and return its original line
/// for the CSV reader to parse normally.
fn read_submission_header(reader: &mut dyn BufRead) -> Result<Vec<u8>, String> {
    let mut line = String::new();
    let mut scanned_bytes = 0usize;

    for _ in 0..MAX_HEADER_SCAN_LINES {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Read ClinVar submission preamble: {error}"))?;
        if bytes == 0 {
            break;
        }
        scanned_bytes = scanned_bytes.saturating_add(bytes);
        if scanned_bytes > MAX_HEADER_SCAN_BYTES {
            return Err(
                "ClinVar submission header was not found within the 1 MiB preamble limit".into(),
            );
        }
        if is_submission_header_candidate(&line) {
            return Ok(line.into_bytes());
        }
    }

    Err(format!(
        "ClinVar submission header with VariationID and SCV was not found within the first {MAX_HEADER_SCAN_LINES} lines"
    ))
}

fn field(record: &StringRecord, column: Option<usize>) -> String {
    column
        .and_then(|index| record.get(index))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn emit_progress(
    app: Option<&tauri::AppHandle>,
    rows: u64,
    rate: u64,
    percent: Option<u8>,
    message: String,
) {
    if let Some(app) = app {
        let _ = app.emit(
            "offline:import_progress",
            serde_json::json!({
                "asset_id": "clinvar_submission_summary",
                "rows_processed": rows,
                "percent": percent,
                "rows_per_second": rate,
                "eta_seconds": null,
                "message": message,
            }),
        );
    }
}

/// Stream ClinVar's per-submission variant-condition assertions into a local sidecar.
/// The current table stays readable until the completed staging table is swapped in.
pub fn import_clinvar_submission_summary(
    data_dir: &Path,
    path: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    let database_path = data_dir.join("clinvar_submissions.db");
    let conn = Connection::open(&database_path)
        .map_err(|error| format!("Open ClinVar submissions database: {error}"))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         DROP TABLE IF EXISTS clinvar_submissions_stage;
         CREATE TABLE clinvar_submissions_stage (
             scv TEXT PRIMARY KEY,
             variation_id TEXT NOT NULL,
             clinical_significance TEXT NOT NULL DEFAULT '',
             date_last_evaluated TEXT NOT NULL DEFAULT '',
             description TEXT NOT NULL DEFAULT '',
             submitted_phenotype_info TEXT NOT NULL DEFAULT '',
             reported_phenotype_info TEXT NOT NULL DEFAULT '',
             review_status TEXT NOT NULL DEFAULT '',
             collection_method TEXT NOT NULL DEFAULT '',
             origin_counts TEXT NOT NULL DEFAULT '',
             submitter TEXT NOT NULL DEFAULT '',
             gene_symbol TEXT NOT NULL DEFAULT '',
             explanation_of_interpretation TEXT NOT NULL DEFAULT '',
             somatic_clinical_impact TEXT NOT NULL DEFAULT '',
             oncogenicity TEXT NOT NULL DEFAULT ''
         );",
    )
    .map_err(|error| format!("Create ClinVar submission staging table: {error}"))?;

    let mut text =
        open_text_auto(path).map_err(|error| format!("Open ClinVar submissions: {error}"))?;
    let header_line = read_submission_header(text.as_mut())?;
    let text = Cursor::new(header_line).chain(text);
    let mut reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .from_reader(text);
    let headers = reader
        .headers()
        .map_err(|error| format!("Read ClinVar submission header: {error}"))?
        .clone();
    let variation_id = find_column(&headers, &["VariationID"])
        .ok_or_else(|| "ClinVar submission summary is missing VariationID".to_string())?;
    let scv = find_column(&headers, &["SCV"])
        .ok_or_else(|| "ClinVar submission summary is missing SCV".to_string())?;

    let columns = [
        find_column(&headers, &["ClinicalSignificance"]),
        find_column(&headers, &["DateLastEvaluated"]),
        find_column(&headers, &["Description"]),
        find_column(&headers, &["SubmittedPhenotypeInfo"]),
        find_column(&headers, &["ReportedPhenotypeInfo"]),
        find_column(&headers, &["ReviewStatus"]),
        find_column(&headers, &["CollectionMethod"]),
        find_column(&headers, &["OriginCounts"]),
        find_column(&headers, &["Submitter"]),
        find_column(&headers, &["SubmittedGeneSymbol"]),
        find_column(&headers, &["ExplanationOfInterpretation"]),
        find_column(&headers, &["SomaticClinicalImpact"]),
        find_column(&headers, &["Oncogenicity"]),
    ];

    let start = Instant::now();
    emit_progress(
        app,
        0,
        0,
        None,
        "Reading ClinVar variant-condition submissions…".into(),
    );
    let mut transaction = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let mut statement = transaction
        .prepare(
            "INSERT OR REPLACE INTO clinvar_submissions_stage (
                scv, variation_id, clinical_significance, date_last_evaluated,
                description, submitted_phenotype_info, reported_phenotype_info,
                review_status, collection_method, origin_counts, submitter,
                gene_symbol, explanation_of_interpretation, somatic_clinical_impact,
                oncogenicity
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .map_err(|error| error.to_string())?;
    let mut row_count = 0u64;

    for row in reader.records() {
        if crate::offline::sync::is_offline_import_cancelled() {
            return Err(
                "ClinVar submission import cancelled; the previous index remains available.".into(),
            );
        }
        let row = row.map_err(|error| format!("Read ClinVar submission row: {error}"))?;
        let variation = field(&row, Some(variation_id));
        let accession = field(&row, Some(scv));
        if variation.is_empty() || accession.is_empty() || variation.eq_ignore_ascii_case("na") {
            continue;
        }
        let values: Vec<String> = columns.iter().map(|column| field(&row, *column)).collect();
        statement
            .execute(params![
                accession, variation, values[0], values[1], values[2], values[3], values[4],
                values[5], values[6], values[7], values[8], values[9], values[10], values[11],
                values[12],
            ])
            .map_err(|error| format!("Index ClinVar submission row: {error}"))?;
        row_count += 1;

        if row_count.is_multiple_of(10_000) {
            drop(statement);
            transaction
                .commit()
                .map_err(|error| format!("Commit ClinVar submission chunk: {error}"))?;
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let rate = (row_count as f64 / elapsed) as u64;
            emit_progress(
                app,
                row_count,
                rate,
                None,
                format!("Indexing ClinVar submissions · {row_count} rows · {rate} rows/s"),
            );
            transaction = conn
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            statement = transaction
                .prepare(
                    "INSERT OR REPLACE INTO clinvar_submissions_stage (
                        scv, variation_id, clinical_significance, date_last_evaluated,
                        description, submitted_phenotype_info, reported_phenotype_info,
                        review_status, collection_method, origin_counts, submitter,
                        gene_symbol, explanation_of_interpretation, somatic_clinical_impact,
                        oncogenicity
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .map_err(|error| error.to_string())?;
        }
    }
    drop(statement);
    transaction
        .commit()
        .map_err(|error| format!("Commit ClinVar submissions: {error}"))?;

    if row_count == 0 {
        conn.execute("DROP TABLE IF EXISTS clinvar_submissions_stage", [])
            .map_err(|error| format!("Discard empty ClinVar submission index: {error}"))?;
        return Err(
            "ClinVar submission summary contained no indexable VariationID/SCV rows; existing index left unchanged."
                .into(),
        );
    }

    conn.execute_batch(
        "BEGIN IMMEDIATE;
         DROP TABLE IF EXISTS clinvar_submissions;
         ALTER TABLE clinvar_submissions_stage RENAME TO clinvar_submissions;
         CREATE INDEX IF NOT EXISTS idx_clinvar_submissions_variation
             ON clinvar_submissions(variation_id);
         CREATE INDEX IF NOT EXISTS idx_clinvar_submissions_classification
             ON clinvar_submissions(clinical_significance);
         COMMIT;",
    )
    .map_err(|error| format!("Publish ClinVar submission index: {error}"))?;

    let elapsed = start.elapsed().as_secs_f64().max(0.001);
    let rate = (row_count as f64 / elapsed) as u64;
    emit_progress(
        app,
        row_count,
        rate,
        Some(100),
        format!("ClinVar submission index ready · {row_count} records"),
    );
    Ok(row_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn imports_condition_specific_submissions_without_exposing_genotypes() {
        let directory = std::env::temp_dir().join(format!(
            "clinvar_submission_import_{}_{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).expect("create synthetic import directory");
        let source = directory.join("submission_summary.txt");
        std::fs::write(
            &source,
            "VariationID\tClinicalSignificance\tDateLastEvaluated\tDescription\tSubmittedPhenotypeInfo\tReportedPhenotypeInfo\tReviewStatus\tCollectionMethod\tOriginCounts\tSubmitter\tSCV\tSubmittedGeneSymbol\tExplanationOfInterpretation\tSomaticClinicalImpact\tOncogenicity\n\
             123\tPathogenic\t2026-01-01\tBasis\tCondition X\tMedGen:C123 (Condition X)\tcriteria provided, single submitter\tclinical testing\t2\tLab A\tSCV000000001.1\tGENE1\t\t\t\n\
             456\tLikely pathogenic\t2026-02-01\t\tCondition Y\tCondition Y\tcriteria provided, multiple submitters\tresearch\t1\tLab B\tSCV000000002.2\tGENE2\t\t\t\n",
        )
        .expect("write synthetic TSV");

        let count = import_clinvar_submission_summary(&directory, &source, None)
            .expect("import synthetic condition records");
        assert_eq!(count, 2);
        let database = Connection::open(directory.join("clinvar_submissions.db"))
            .expect("open generated submission index");
        let row: (String, String, String) = database
            .query_row(
                "SELECT clinical_significance, reported_phenotype_info, review_status
                 FROM clinvar_submissions WHERE scv = 'SCV000000001.1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read indexed synthetic row");
        assert_eq!(row.0, "Pathogenic");
        assert!(row.1.contains("Condition X"));
        assert!(row.2.contains("single submitter"));

        drop(database);
        std::fs::remove_dir_all(directory).expect("remove synthetic import directory");
    }

    #[test]
    fn imports_gzipped_submission_summary_after_ncbi_metadata_preamble() {
        let directory = std::env::temp_dir().join(format!(
            "clinvar_submission_preamble_{}_{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).expect("create synthetic import directory");
        let source = directory.join("submission_summary.txt.gz");
        let file = std::fs::File::create(&source).expect("create synthetic gzip source");
        let mut encoder = GzEncoder::new(file, Compression::default());
        for index in 0..18 {
            writeln!(encoder, "##ClinVar metadata line {index}")
                .expect("write synthetic metadata line");
        }
        encoder
            .write_all(
                b"#VariationID\tSCV\tClinicalSignificance\n123\tSCV000000123.1\tPathogenic\n",
            )
            .expect("write synthetic submission rows");
        encoder.finish().expect("finish synthetic gzip source");

        let count = import_clinvar_submission_summary(&directory, &source, None)
            .expect("import gzipped summary after NCBI metadata");
        assert_eq!(count, 1);
        let database = Connection::open(directory.join("clinvar_submissions.db"))
            .expect("open generated submission index");
        let row: (String, String) = database
            .query_row(
                "SELECT variation_id, clinical_significance
                 FROM clinvar_submissions WHERE scv = 'SCV000000123.1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read indexed synthetic submission");
        assert_eq!(row, ("123".into(), "Pathogenic".into()));

        drop(database);
        std::fs::remove_dir_all(directory).expect("remove synthetic import directory");
    }

    #[test]
    fn empty_submission_summary_does_not_replace_existing_index() {
        let directory = std::env::temp_dir().join(format!(
            "clinvar_submission_empty_{}_{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).expect("create synthetic import directory");
        let source = directory.join("submission_summary.txt");
        std::fs::write(
            &source,
            "#VariationID\tSCV\tClinicalSignificance\n123\tSCV000000123.1\tPathogenic\n",
        )
        .expect("write initial synthetic TSV");
        assert_eq!(
            import_clinvar_submission_summary(&directory, &source, None)
                .expect("import initial synthetic submission"),
            1
        );

        std::fs::write(&source, "#VariationID\tSCV\tClinicalSignificance\n")
            .expect("write empty synthetic TSV");
        let error = import_clinvar_submission_summary(&directory, &source, None)
            .expect_err("refuse to publish an empty submission index");
        assert!(error.contains("no indexable VariationID/SCV rows"));

        let database = Connection::open(directory.join("clinvar_submissions.db"))
            .expect("open existing submission index");
        let count: i64 = database
            .query_row("SELECT COUNT(*) FROM clinvar_submissions", [], |row| {
                row.get(0)
            })
            .expect("count preserved submission rows");
        assert_eq!(count, 1);

        drop(database);
        std::fs::remove_dir_all(directory).expect("remove synthetic import directory");
    }
}
