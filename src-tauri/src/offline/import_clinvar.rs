// ./src-tauri/src/offline/import_clinvar.rs
use crate::research::util::normalize_rsid;
use flate2::read::GzDecoder;
use rusqlite::{params, Connection};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn find_col(headers: &[String], candidates: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let u = h.trim().trim_start_matches('#').to_uppercase();
        candidates
            .iter()
            .any(|c| u == c.to_uppercase() || u.replace(' ', "") == c.to_uppercase().replace(' ', ""))
    })
}

use tauri::Emitter;

pub fn import_clinvar_variant_summary(
    conn: &Connection,
    path: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("Open ClinVar summary: {e}"))?;
    let is_gz = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("gz"));

    let reader: Box<dyn BufRead> = if is_gz {
        Box::new(BufReader::new(GzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    };

    let mut lines = reader.lines();

    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "ClinVar summary empty".to_string())?;

    let headers: Vec<String> = header.split('\t').map(|s| s.to_string()).collect();
    let rs_idx = find_col(&headers, &["RS# (dbSNP)", "RS#", "RSID", "SNP"]).ok_or_else(|| {
        "ClinVar summary missing RS# column".to_string()
    })?;

    let allele_idx = find_col(&headers, &["#AlleleID", "AlleleID", "ALLELEID"]);
    let var_idx = find_col(&headers, &["VariationID", "VARIATION ID"]);
    let type_idx = find_col(&headers, &["Type", "TYPE"]);
    let name_idx = find_col(&headers, &["Name", "NAME"]);
    let gene_idx = find_col(&headers, &["GeneSymbol", "GENE", "GENE SYMBOL"]);
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
    let scvs_idx = find_col(&headers, &["SCVsForAggregateGermlineClassification", "SCVSFORAGGREGATEGERMLINECLASSIFICATION"]);

    conn.execute("DELETE FROM reference.clinvar_reference", [])
        .map_err(|e| e.to_string())?;

    let mut tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line_res in lines {
        let line = line_res.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let rs_field = parts.get(rs_idx).copied().unwrap_or("").trim();
        if rs_field.is_empty() || rs_field == "-" || rs_field == "na" {
            continue;
        }

        // Normalize ClinVar RSID value. It's typically numeric in the file, but let's parse/normalize:
        let rsid = match normalize_rsid(rs_field) {
            Some(r) => r,
            None => continue,
        };

        let allele_id = allele_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let variation_id = var_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let val_type = type_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let name = name_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let gene_symbol = gene_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let clinical_significance = sig_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let clin_sig_simple = sig_simple_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let last_evaluated = eval_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let rcv_accession = rcv_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let phenotype_ids = phen_ids_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let phenotype_list = phen_list_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let origin_simple = origin_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let assembly = asm_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let chromosome = chrom_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        
        let start = start_idx.and_then(|i| parts.get(i)).and_then(|s| s.parse::<i64>().ok());
        let stop = stop_idx.and_then(|i| parts.get(i)).and_then(|s| s.parse::<i64>().ok());
        
        let review_status = review_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let number_submitters = submitters_idx.and_then(|i| parts.get(i)).and_then(|s| s.parse::<i64>().ok());
        let position_vcf = pos_vcf_idx.and_then(|i| parts.get(i)).and_then(|s| s.parse::<i64>().ok());
        
        let reference_allele_vcf = ref_vcf_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let alternate_allele_vcf = alt_vcf_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");
        let scvs_for_aggregate_germline_classification = scvs_idx.and_then(|i| parts.get(i)).map(|s| s.trim()).unwrap_or("");

        // Keep both GRCh37 and GRCh38 assemblies
        let assembly_norm = if assembly.to_uppercase().contains("37") || assembly.to_uppercase().contains("19") {
            "GRCh37"
        } else if assembly.to_uppercase().contains("38") {
            "GRCh38"
        } else {
            assembly
        };

        // Backward compatibility mapping fields:
        let gene = gene_symbol;
        let conditions = phenotype_list;
        let relevant_allele = if !alternate_allele_vcf.is_empty() {
            alternate_allele_vcf
        } else {
            reference_allele_vcf
        };

        tx.execute(
            "INSERT OR REPLACE INTO reference.clinvar_reference (
                rsid, allele_id, variation_id, type, name, gene_symbol,
                clinical_significance, clin_sig_simple, last_evaluated, rcv_accession,
                phenotype_ids, phenotype_list, origin_simple, assembly, chromosome,
                start, stop, review_status, number_submitters, position_vcf,
                reference_allele_vcf, alternate_allele_vcf, scvs_for_aggregate_germline_classification,
                gene, conditions, relevant_allele
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
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
                relevant_allele
            ],
        )
        .map_err(|e| e.to_string())?;

        count += 1;
        if count.is_multiple_of(50_000) {
            tx.commit().map_err(|e| e.to_string())?;
            if let Some(handle) = app {
                let _ = handle.emit(
                    "offline:import_progress",
                    serde_json::json!({
                        "asset_id": "clinvar_variant_summary",
                        "rows_processed": count,
                        "message": format!("Importing ClinVar: processed {} rows...", count)
                    }),
                );
            }
            tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
