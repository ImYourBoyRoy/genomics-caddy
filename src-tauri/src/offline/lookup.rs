// ./src-tauri/src/offline/lookup.rs
//! Local-first lookups used by enrichment before live APIs.

use crate::research::evidence::ncbi_context::ClinvarLiveContext;
use crate::research::util::normalize_rsid;
use rusqlite::{Connection, params};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct ClinvarLocalRecord {
    pub rsid: String,
    pub gene: Option<String>,
    pub significance: String,
    pub conditions: String,
    pub relevant_allele: String,
    pub review_status: String,
    pub variation_id: Option<String>,
}

pub fn lookup_clinvar_local_all(conn: &Connection, rsid: &str) -> Vec<ClinvarLocalRecord> {
    let Some(rsid_norm) = normalize_rsid(rsid) else {
        return vec![];
    };
    let mut stmt = match conn.prepare(
        "SELECT rsid, gene, clinical_significance, conditions, relevant_allele,
                review_status, variation_id
         FROM clinvar_reference
         WHERE UPPER(rsid) = UPPER(?)
         ORDER BY variation_id, gene, clinical_significance, review_status",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return vec![],
    };
    let rows = match stmt.query_map(params![rsid_norm], |row| {
        Ok(ClinvarLocalRecord {
            rsid: row.get(0)?,
            gene: row.get(1)?,
            significance: row.get(2)?,
            conditions: row.get(3)?,
            relevant_allele: row.get(4)?,
            review_status: row.get(5)?,
            variation_id: row.get(6)?,
        })
    }) {
        Ok(rows) => rows,
        Err(_) => return vec![],
    };
    rows.flatten()
        .filter(|record| !record.significance.is_empty() || !record.conditions.is_empty())
        .collect()
}

pub fn lookup_clinvar_local(conn: &Connection, rsid: &str) -> Option<ClinvarLocalRecord> {
    lookup_clinvar_local_all(conn, rsid).into_iter().next()
}

pub fn clinvar_local_to_live_context(rec: &ClinvarLocalRecord) -> ClinvarLiveContext {
    let sig = if rec.significance.is_empty() {
        None
    } else {
        Some(rec.significance.clone())
    };
    let narrative = match (&sig, &rec.conditions) {
        (Some(s), c) if !c.is_empty() => {
            Some(format!("ClinVar (local): {} — {} [{}]", rec.rsid, s, c))
        }
        (Some(s), _) => Some(format!("ClinVar (local): {} — {}", rec.rsid, s)),
        _ => None,
    };
    ClinvarLiveContext {
        significance: sig,
        condition: if rec.conditions.is_empty() {
            None
        } else {
            Some(rec.conditions.clone())
        },
        variation_id: rec.variation_id.clone(),
        review_status: if rec.review_status.is_empty() {
            None
        } else {
            Some(rec.review_status.clone())
        },
        narrative,
        provenance: json!({
            "queried": true,
            "hit": true,
            "source": "clinvar_reference_local",
            "local": true,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PharmgkbLocalRecord {
    pub gene: Option<String>,
    pub drug: Option<String>,
    pub phenotype: Option<String>,
    pub evidence_level: Option<String>,
}

pub fn lookup_pharmgkb_local_records(
    conn: &Connection,
    rsid: &str,
) -> Vec<PharmgkbLocalRecord> {
    let rsid_norm = normalize_rsid(rsid).unwrap_or_else(|| rsid.to_uppercase());
    let mut stmt = match conn.prepare(
        "SELECT gene, drug, phenotype, evidence_level
         FROM pharmgkb_clinical_variants
         WHERE UPPER(rsid) = UPPER(?)
         ORDER BY drug, phenotype, evidence_level",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return vec![],
    };
    let rows = match stmt.query_map(params![rsid_norm], |row| {
        Ok(PharmgkbLocalRecord {
            gene: row.get(0)?,
            drug: row.get(1)?,
            phenotype: row.get(2)?,
            evidence_level: row.get(3)?,
        })
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    rows.flatten().collect()
}

pub fn lookup_pharmgkb_local(conn: &Connection, rsid: &str) -> (Vec<String>, u32, bool) {
    let records = lookup_pharmgkb_local_records(conn, rsid);
    let mut drugs = Vec::new();
    for record in &records {
        if let Some(drug) = &record.drug
            && !drugs.contains(drug)
        {
            drugs.push(drug.clone());
        }
    }
    let count = records.len().min(u32::MAX as usize) as u32;
    (drugs, count, !records.is_empty())
}

pub fn lookup_mane_transcript(conn: &Connection, gene: &str) -> Option<String> {
    conn.query_row(
        "SELECT ensembl_transcript FROM mane_transcripts WHERE UPPER(gene_symbol) = UPPER(?) LIMIT 1",
        params![gene],
        |row| row.get(0),
    )
    .ok()
    .flatten()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClinGenLocalRecord {
    pub hgnc_id: Option<String>,
    pub gene_symbol: String,
    pub disease_label: String,
    pub classification: Option<String>,
    pub mode_of_inheritance: Option<String>,
    pub report_url: Option<String>,
}

pub fn lookup_clingen_validity_records(
    conn: &Connection,
    gene: &str,
) -> Vec<ClinGenLocalRecord> {
    let mut stmt = match conn.prepare(
        "SELECT hgnc_id, gene_symbol, disease_label, classification, moi, report_url
         FROM clingen_gene_validity
         WHERE UPPER(gene_symbol) = UPPER(?)
         ORDER BY disease_label, classification",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return vec![],
    };
    let rows = match stmt.query_map(params![gene], |row| {
        Ok(ClinGenLocalRecord {
            hgnc_id: row.get(0)?,
            gene_symbol: row.get(1)?,
            disease_label: row.get(2)?,
            classification: row.get(3)?,
            mode_of_inheritance: row.get(4)?,
            report_url: row.get(5)?,
        })
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    rows.flatten().collect()
}

pub fn lookup_clingen_validity(conn: &Connection, gene: &str) -> Vec<String> {
    lookup_clingen_validity_records(conn, gene)
        .into_iter()
        .map(|record| {
            if let Some(classification) = record.classification {
                format!("{} ({classification})", record.disease_label)
            } else {
                record.disease_label
            }
        })
        .collect()
}

pub fn resolve_rsid_alias(conn: &Connection, rsid: &str) -> Option<String> {
    let rsid_norm = normalize_rsid(rsid)?;
    conn.query_row(
        "SELECT merged_into, withdrawn FROM rsid_aliases WHERE UPPER(rsid) = UPPER(?)",
        params![rsid_norm],
        |row| {
            let withdrawn: i64 = row.get(1)?;
            if withdrawn != 0 {
                return Ok(None);
            }
            row.get::<_, Option<String>>(0)
        },
    )
    .ok()
    .flatten()
    .and_then(|s| normalize_rsid(&s))
}

pub fn lookup_variant_locus_local(
    conn: &Connection,
    rsid: &str,
) -> Option<(String, i64, String, String)> {
    let rsid_norm = normalize_rsid(rsid)?;
    conn.query_row(
        "SELECT chrom, pos, ref_allele, alt_allele FROM variant_locus WHERE UPPER(rsid) = UPPER(?) LIMIT 1",
        params![rsid_norm],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )
    .ok()
}

pub fn offline_clinvar_available(conn: &Connection) -> bool {
    conn.query_row("SELECT COUNT(*) FROM clinvar_reference", [], |row| {
        row.get::<_, i64>(0)
    })
    .unwrap_or(0)
        > 0
}

pub fn offline_pharmgkb_available(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM pharmgkb_clinical_variants
         WHERE TRIM(COALESCE(drug, '')) <> ''
            OR TRIM(COALESCE(phenotype, '')) <> ''",
        [],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_catalog_lookups_do_not_truncate_related_records() {
        let conn = Connection::open_in_memory().expect("open lookup fixture");
        conn.execute_batch(
            "
            CREATE TABLE clinvar_reference (
                rsid TEXT, gene TEXT, clinical_significance TEXT, conditions TEXT,
                relevant_allele TEXT, review_status TEXT, variation_id TEXT
            );
            CREATE TABLE pharmgkb_clinical_variants (
                rsid TEXT, gene TEXT, drug TEXT, phenotype TEXT, evidence_level TEXT
            );
            CREATE TABLE clingen_gene_validity (
                hgnc_id TEXT, gene_symbol TEXT, disease_label TEXT,
                classification TEXT, moi TEXT, report_url TEXT
            );
            ",
        )
        .expect("create lookup tables");
        conn.execute(
            "INSERT INTO clinvar_reference
             VALUES ('rs123', 'GENE1', 'Pathogenic', 'Condition A', 'A', 'reviewed', '1'),
                    ('rs123', 'GENE1', 'Benign', 'Condition B', 'G', 'provided', '2')",
            [],
        )
        .expect("seed ClinVar rows");
        conn.execute(
            "INSERT INTO pharmgkb_clinical_variants
             VALUES ('rs123', 'GENE1', 'Drug A', 'Phenotype A', '1A'),
                    ('rs123', 'GENE1', 'Drug B', 'Phenotype B', '2A')",
            [],
        )
        .expect("seed ClinPGx rows");
        conn.execute(
            "INSERT INTO clingen_gene_validity
             VALUES ('HGNC:1', 'GENE1', 'Condition A', 'Definitive', 'Autosomal dominant', 'https://example.test/a'),
                    ('HGNC:1', 'GENE1', 'Condition B', 'Limited', 'Autosomal recessive', 'https://example.test/b')",
            [],
        )
        .expect("seed ClinGen rows");

        assert_eq!(lookup_clinvar_local_all(&conn, "rs123").len(), 2);
        assert_eq!(lookup_pharmgkb_local_records(&conn, "rs123").len(), 2);
        assert_eq!(lookup_clingen_validity_records(&conn, "GENE1").len(), 2);
        assert!(offline_pharmgkb_available(&conn));
    }

    #[test]
    fn empty_clinpgx_annotation_rows_are_not_reported_as_available() {
        let conn = Connection::open_in_memory().expect("open semantic lookup fixture");
        conn.execute(
            "CREATE TABLE pharmgkb_clinical_variants (
                rsid TEXT, gene TEXT, drug TEXT, phenotype TEXT, evidence_level TEXT
            )",
            [],
        )
        .expect("create ClinPGx semantic table");
        conn.execute(
            "INSERT INTO pharmgkb_clinical_variants VALUES ('rs123', 'GENE1', '', '', '1A')",
            [],
        )
        .expect("seed empty ClinPGx annotation");

        assert!(!offline_pharmgkb_available(&conn));
    }
}
