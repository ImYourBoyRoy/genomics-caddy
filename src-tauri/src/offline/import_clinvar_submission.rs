use crate::offline::compress::open_text_auto;
use csv::{ReaderBuilder, StringRecord};
use rusqlite::{Connection, params};
use std::path::Path;
use std::time::Instant;
use tauri::Emitter;

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
        names.iter().any(|name| normalized == normalize_header(name))
    })
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

    let text = open_text_auto(path).map_err(|error| format!("Open ClinVar submissions: {error}"))?;
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
            return Err("ClinVar submission import cancelled; the previous index remains available.".into());
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
                accession,
                variation,
                values[0],
                values[1],
                values[2],
                values[3],
                values[4],
                values[5],
                values[6],
                values[7],
                values[8],
                values[9],
                values[10],
                values[11],
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
}
