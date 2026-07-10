// ./src-tauri/src/offline/import_clinvar.rs
use super::compress::open_text_auto;
use crate::research::util::normalize_rsid;
use rayon::prelude::*;
use rusqlite::{Connection, params};
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn find_col(headers: &[String], candidates: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let u = h.trim().trim_start_matches('#').to_uppercase();
        candidates.iter().any(|c| {
            u == c.to_uppercase() || u.replace(' ', "") == c.to_uppercase().replace(' ', "")
        })
    })
}

use tauri::Emitter;

struct ClinVarRow {
    rsid: String,
    allele_id: String,
    variation_id: String,
    val_type: String,
    name: String,
    gene_symbol: String,
    clinical_significance: String,
    clin_sig_simple: String,
    last_evaluated: String,
    rcv_accession: String,
    phenotype_ids: String,
    phenotype_list: String,
    origin_simple: String,
    assembly_norm: String,
    chromosome: String,
    start: Option<i64>,
    stop: Option<i64>,
    review_status: String,
    number_submitters: Option<i64>,
    position_vcf: Option<i64>,
    reference_allele_vcf: String,
    alternate_allele_vcf: String,
    scvs_for_aggregate_germline_classification: String,
    gene: String,
    conditions: String,
    relevant_allele: String,
}

pub fn import_clinvar_variant_summary(
    conn: &Connection,
    path: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    let reader: Box<dyn BufRead> =
        open_text_auto(path).map_err(|error| format!("Open ClinVar summary: {error}"))?;

    let mut lines = reader.lines();

    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "ClinVar summary empty".to_string())?;

    let headers: Vec<String> = header.split('\t').map(|s| s.to_string()).collect();
    let rs_idx = find_col(&headers, &["RS# (dbSNP)", "RS#", "RSID", "SNP"])
        .ok_or_else(|| "ClinVar summary missing RS# column".to_string())?;

    let allele_idx = find_col(&headers, &["#AlleleID", "AlleleID", "ALLELEID"]);
    let var_idx = find_col(&headers, &["VariationID", "VARIATION ID"]);
    let type_idx = find_col(&headers, &["Type", "TYPE"]);
    let name_idx = find_col(&headers, &["Name", "NAME"]);
    let gene_symbol_idx = find_col(&headers, &["GeneSymbol", "GENE", "GENE SYMBOL"]);
    let sig_idx = find_col(&headers, &["ClinicalSignificance", "CLINICAL SIGNIFICANCE"]);
    let sig_simple_idx = find_col(&headers, &["ClinSigSimple", "CLINSIGSIMPLE"]);
    let eval_idx = find_col(&headers, &["LastEvaluated", "LAST EVALUATED"]);
    let rcv_idx = find_col(&headers, &["RCVaccession", "RCVACCESSION"]);
    let phen_ids_idx = find_col(&headers, &["PhenotypeIDS", "PHENOTYPEIDS"]);
    let phen_list_idx = find_col(&headers, &["PhenotypeList", "PHENOTYPELIST"]);
    let origin_idx = find_col(&headers, &["OriginSimple", "ORIGINSIMPLE"]);
    let asm_idx = find_col(&headers, &["Assembly", "ASSEMBLY"]);
    let chrom_idx = find_col(&headers, &["Chromosome", "CHROMOSOME"]);
    let start_idx = find_col(&headers, &["Start", "START"]);
    let stop_idx = find_col(&headers, &["Stop", "STOP"]);
    let review_idx = find_col(&headers, &["ReviewStatus", "REVIEW STATUS"]);
    let submitters_idx = find_col(&headers, &["NumberSubmitters", "NUMBERSUBMITTERS"]);
    let pos_vcf_idx = find_col(&headers, &["PositionVCF", "POSITIONVCF"]);
    let ref_vcf_idx = find_col(&headers, &["ReferenceAlleleVCF", "REFERENCEALLELEVCF"]);
    let alt_vcf_idx = find_col(&headers, &["AlternateAlleleVCF", "ALTERNATEALLELEVCF"]);
    let scvs_idx = find_col(
        &headers,
        &[
            "SCVsForAggregateGermlineClassification",
            "SCVSFORAGGREGATEGERMLINECLASSIFICATION",
        ],
    );

    // Locate active clinvar.db file path
    let clinvar_path_str: String = conn
        .query_row(
            "SELECT file FROM pragma_database_list() WHERE name = 'clinvar'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Query clinvar.db path: {e}"))?;

    let clinvar_path = PathBuf::from(&clinvar_path_str);
    let clinvar_dir = clinvar_path.parent().unwrap_or_else(|| Path::new("."));
    let staging_path = clinvar_dir.join("clinvar_staging.db");

    if staging_path.is_file() {
        let _ = std::fs::remove_file(&staging_path);
    }

    // Open staging database connection
    let staging_conn =
        Connection::open(&staging_path).map_err(|e| format!("Open staging DB: {e}"))?;

    // Apply ingestion write performance PRAGMAs
    staging_conn.execute("PRAGMA synchronous = OFF", []).ok();
    staging_conn
        .execute("PRAGMA journal_mode = MEMORY", [])
        .ok();
    staging_conn.execute("PRAGMA cache_size = 100000", []).ok();

    staging_conn
        .execute(
            "CREATE TABLE IF NOT EXISTS clinvar_reference (
            rsid TEXT NOT NULL,
            allele_id TEXT NOT NULL DEFAULT '',
            variation_id TEXT NOT NULL DEFAULT '',
            type TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL DEFAULT '',
            gene_symbol TEXT NOT NULL DEFAULT '',
            clinical_significance TEXT NOT NULL DEFAULT '',
            clin_sig_simple TEXT NOT NULL DEFAULT '',
            last_evaluated TEXT NOT NULL DEFAULT '',
            rcv_accession TEXT NOT NULL DEFAULT '',
            phenotype_ids TEXT NOT NULL DEFAULT '',
            phenotype_list TEXT NOT NULL DEFAULT '',
            origin_simple TEXT NOT NULL DEFAULT '',
            assembly TEXT NOT NULL DEFAULT '',
            chromosome TEXT NOT NULL DEFAULT '',
            start INTEGER,
            stop INTEGER,
            review_status TEXT NOT NULL DEFAULT '',
            number_submitters INTEGER,
            position_vcf INTEGER,
            reference_allele_vcf TEXT NOT NULL DEFAULT '',
            alternate_allele_vcf TEXT NOT NULL DEFAULT '',
            scvs_for_aggregate_germline_classification TEXT NOT NULL DEFAULT '',
            gene TEXT NOT NULL DEFAULT '',
            conditions TEXT NOT NULL DEFAULT '',
            relevant_allele TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (rsid, assembly, gene_symbol, variation_id)
        )",
            [],
        )
        .map_err(|e| format!("Create staging schema: {e}"))?;

    let start_time = std::time::Instant::now();
    let total_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut bytes_processed = 0u64;

    let parse_line = |line: &str| -> Option<ClinVarRow> {
        let parts: Vec<&str> = line.split('\t').collect();
        let rs_field = parts.get(rs_idx).copied().unwrap_or("").trim();
        if rs_field.is_empty() || rs_field == "-" || rs_field == "na" {
            return None;
        }

        let rsid = normalize_rsid(rs_field)?;
        let allele_id = allele_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let variation_id = var_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let val_type = type_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let name = name_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let gene_symbol = gene_symbol_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let clinical_significance = sig_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let clin_sig_simple = sig_simple_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let last_evaluated = eval_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let rcv_accession = rcv_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let phenotype_ids = phen_ids_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let phenotype_list = phen_list_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let origin_simple = origin_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let assembly = asm_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let chromosome = chrom_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();

        let start = start_idx
            .and_then(|i| parts.get(i))
            .and_then(|s| s.parse::<i64>().ok());
        let stop = stop_idx
            .and_then(|i| parts.get(i))
            .and_then(|s| s.parse::<i64>().ok());

        let review_status = review_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let number_submitters = submitters_idx
            .and_then(|i| parts.get(i))
            .and_then(|s| s.parse::<i64>().ok());
        let position_vcf = pos_vcf_idx
            .and_then(|i| parts.get(i))
            .and_then(|s| s.parse::<i64>().ok());

        let reference_allele_vcf = ref_vcf_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let alternate_allele_vcf = alt_vcf_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        let scvs_for_aggregate_germline_classification = scvs_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();

        let assembly_norm =
            if assembly.to_uppercase().contains("37") || assembly.to_uppercase().contains("19") {
                "GRCh37".to_string()
            } else if assembly.to_uppercase().contains("38") {
                "GRCh38".to_string()
            } else {
                assembly
            };

        let gene = gene_symbol.clone();
        let conditions = phenotype_list.clone();
        let relevant_allele = if !alternate_allele_vcf.is_empty() {
            alternate_allele_vcf.clone()
        } else {
            reference_allele_vcf.clone()
        };

        Some(ClinVarRow {
            rsid,
            allele_id,
            variation_id,
            val_type,
            name,
            gene_symbol,
            clinical_significance,
            clin_sig_simple,
            last_evaluated,
            rcv_accession,
            phenotype_ids,
            phenotype_list,
            origin_simple,
            assembly_norm,
            chromosome,
            start,
            stop,
            review_status,
            number_submitters,
            position_vcf,
            reference_allele_vcf,
            alternate_allele_vcf,
            scvs_for_aggregate_germline_classification,
            gene,
            conditions,
            relevant_allele,
        })
    };

    let mut tx = staging_conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let mut count = 0u64;
    let mut chunk = Vec::with_capacity(20_000);

    for line_res in lines {
        if crate::offline::sync::is_offline_import_cancelled() {
            return Err("ClinVar import cancelled by user.".into());
        }
        let line = line_res.map_err(|e| e.to_string())?;
        let line_len = line.len() as u64 + 1;
        bytes_processed += line_len;

        if line.is_empty() {
            continue;
        }
        chunk.push(line);

        if chunk.len() >= 20_000 {
            let parsed_rows: Vec<ClinVarRow> =
                chunk.par_iter().filter_map(|l| parse_line(l)).collect();

            for row in parsed_rows {
                tx.execute(
                    "INSERT OR REPLACE INTO clinvar_reference (
                        rsid, allele_id, variation_id, type, name, gene_symbol,
                        clinical_significance, clin_sig_simple, last_evaluated, rcv_accession,
                        phenotype_ids, phenotype_list, origin_simple, assembly, chromosome,
                        start, stop, review_status, number_submitters, position_vcf,
                        reference_allele_vcf, alternate_allele_vcf, scvs_for_aggregate_germline_classification,
                        gene, conditions, relevant_allele
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        row.rsid, row.allele_id, row.variation_id, row.val_type, row.name, row.gene_symbol,
                        row.clinical_significance, row.clin_sig_simple, row.last_evaluated, row.rcv_accession,
                        row.phenotype_ids, row.phenotype_list, row.origin_simple, row.assembly_norm, row.chromosome,
                        row.start, row.stop, row.review_status, row.number_submitters, row.position_vcf,
                        row.reference_allele_vcf, row.alternate_allele_vcf, row.scvs_for_aggregate_germline_classification,
                        row.gene, row.conditions, row.relevant_allele
                    ],
                ).map_err(|e| e.to_string())?;
                count += 1;
            }

            tx.commit().map_err(|e| e.to_string())?;
            if let Some(handle) = app {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    count as f64 / elapsed
                } else {
                    0.0
                };
                let percent = if total_bytes > 0 {
                    ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u64
                } else {
                    0
                };
                // Cap in-flight progress at 99%; emit 100% only after commit/swap.
                let percent_bounded = percent.min(99);
                let eta_seconds = if speed > 0.0 && total_bytes > bytes_processed {
                    let remaining_bytes = total_bytes - bytes_processed;
                    let avg_bytes_per_row = bytes_processed as f64 / count as f64;
                    let remaining_rows = remaining_bytes as f64 / avg_bytes_per_row;
                    Some((remaining_rows / speed) as u64)
                } else {
                    None
                };

                let _ = handle.emit(
                    "offline:import_progress",
                    serde_json::json!({
                        "asset_id": "clinvar_variant_summary",
                        "rows_processed": count,
                        "percent": percent_bounded,
                        "rows_per_second": speed as u64,
                        "eta_seconds": eta_seconds,
                        "message": format!(
                            "Importing ClinVar: {}% · {} rows ({:.0} rows/s)",
                            percent_bounded, count, speed
                        )
                    }),
                );
            }
            tx = staging_conn
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            chunk.clear();
        }
    }

    // Process remainder
    if !chunk.is_empty() {
        let parsed_rows: Vec<ClinVarRow> = chunk.par_iter().filter_map(|l| parse_line(l)).collect();

        for row in parsed_rows {
            tx.execute(
                "INSERT OR REPLACE INTO clinvar_reference (
                    rsid, allele_id, variation_id, type, name, gene_symbol,
                    clinical_significance, clin_sig_simple, last_evaluated, rcv_accession,
                    phenotype_ids, phenotype_list, origin_simple, assembly, chromosome,
                    start, stop, review_status, number_submitters, position_vcf,
                    reference_allele_vcf, alternate_allele_vcf, scvs_for_aggregate_germline_classification,
                    gene, conditions, relevant_allele
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    row.rsid, row.allele_id, row.variation_id, row.val_type, row.name, row.gene_symbol,
                    row.clinical_significance, row.clin_sig_simple, row.last_evaluated, row.rcv_accession,
                    row.phenotype_ids, row.phenotype_list, row.origin_simple, row.assembly_norm, row.chromosome,
                    row.start, row.stop, row.review_status, row.number_submitters, row.position_vcf,
                    row.reference_allele_vcf, row.alternate_allele_vcf, row.scvs_for_aggregate_germline_classification,
                    row.gene, row.conditions, row.relevant_allele
                ],
            ).map_err(|e| e.to_string())?;
            count += 1;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Secondary indexes for report/agent lookups (PK alone is composite).
    staging_conn
        .execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_clinvar_reference_rsid ON clinvar_reference(rsid);
            CREATE INDEX IF NOT EXISTS idx_clinvar_reference_gene ON clinvar_reference(gene_symbol);
            CREATE INDEX IF NOT EXISTS idx_clinvar_reference_varid ON clinvar_reference(variation_id);
            CREATE INDEX IF NOT EXISTS idx_clinvar_reference_coords
                ON clinvar_reference(assembly, chromosome, start);
            ",
        )
        .map_err(|e| format!("ClinVar index create failed: {e}"))?;

    if let Some(handle) = app {
        let _ = handle.emit(
            "offline:import_progress",
            serde_json::json!({
                "asset_id": "clinvar_variant_summary",
                "rows_processed": count,
                "percent": 100,
                "rows_per_second": 0,
                "eta_seconds": null,
                "message": format!("ClinVar import complete · {} rows indexed", count)
            }),
        );
    }

    // Drop staging connection to release file lock before swap
    drop(staging_conn);

    // Detach current clinvar database on the main connection to release lock on clinvar.db
    let _ = conn.execute("DETACH DATABASE clinvar", []);

    // Swap files atomically
    if clinvar_path.is_file() {
        let _ = std::fs::remove_file(&clinvar_path);
    }
    std::fs::rename(&staging_path, &clinvar_path)
        .map_err(|e| format!("Staging swap rename failed: {e}"))?;

    // Re-attach new clinvar database to connection
    let attach_clinvar = format!(
        "ATTACH DATABASE '{}' AS clinvar",
        clinvar_path.to_string_lossy().replace('\\', "/")
    );
    conn.execute(&attach_clinvar, [])
        .map_err(|e| format!("Re-attach clinvar failed: {e}"))?;

    Ok(count)
}
