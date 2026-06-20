// ./src-tauri/src/research/markers.rs
use super::types::*;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
pub fn score_marker_significance(
    clinvar_sig: Option<&str>,
    gwas_strict_count: u32,
    gnomad_af: Option<f64>,
    best_pvalue: Option<f64>,
) -> f32 {
    let mut score: f32 = 0.0;
    match clinvar_sig {
        Some(s) if s.contains("Pathogenic") && !s.contains("Likely") => score += 0.5,
        Some(s) if s.contains("Likely pathogenic") => score += 0.4,
        Some(s) if s.contains("Risk factor") || s.contains("Association") => score += 0.2,
        Some(s) if s.contains("Uncertain") => score += 0.05,
        _ => {}
    }
    if gwas_strict_count > 0 {
        score += 0.35;
    } else if best_pvalue.is_some() {
        score += 0.12;
    }
    if let Some(p) = best_pvalue {
        if p < 5e-8 {
            score += 0.25;
        } else if p < 1e-5 {
            score += 0.15;
        } else if p < 1e-3 {
            score += 0.05;
        }
    }
    if let Some(af) = gnomad_af {
        if af < 0.001 {
            score += 0.2;
        } else if af < 0.01 {
            score += 0.1;
        } else if af < 0.05 {
            score += 0.05;
        }
    }
    score.min(1.0)
}


const EVIDENCE_GENE_SUBQUERY: &str = "
    SELECT rsid, MIN(gene) AS gene
    FROM evidence_library
    WHERE gene IS NOT NULL AND TRIM(gene) != ''
    GROUP BY rsid
";

fn row_to_scored_marker(
    rsid: String,
    chromosome: String,
    allele1: String,
    allele2: String,
    clinvar_sig: Option<String>,
    gene: Option<String>,
) -> ScoredMarker {
    ScoredMarker {
        significance_score: score_marker_significance(clinvar_sig.as_deref(), 0, None, None),
        rsid,
        gene,
        chromosome,
        allele1,
        allele2,
        clinvar_sig,
    }
}

fn merge_scored_marker(map: &mut HashMap<String, ScoredMarker>, marker: ScoredMarker) {
    map.entry(marker.rsid.clone())
        .and_modify(|existing| {
            if marker.significance_score > existing.significance_score {
                existing.significance_score = marker.significance_score;
            }
            if existing.gene.is_none() {
                existing.gene = marker.gene.clone();
            }
            if existing.clinvar_sig.is_none() {
                existing.clinvar_sig = marker.clinvar_sig.clone();
            }
        })
        .or_insert(marker);
}

fn load_curated_markers(conn: &Connection, sample_id: i64) -> Result<Vec<ScoredMarker>, String> {
    let sql = format!(
        "SELECT g.rsid, g.chromosome, g.allele1, g.allele2, c.clinical_significance,
                COALESCE(NULLIF(TRIM(c.gene), ''), NULLIF(TRIM(e.gene), '')) AS gene
         FROM genotypes g
         LEFT JOIN clinvar_reference c ON g.rsid = c.rsid
         LEFT JOIN ({EVIDENCE_GENE_SUBQUERY}) e ON g.rsid = e.rsid
         WHERE g.sample_id = ?
           AND (g.rsid IN (SELECT DISTINCT rsid FROM clinvar_reference)
                OR g.rsid IN (SELECT DISTINCT rsid FROM evidence_library))"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id], |row| {
            Ok(row_to_scored_marker(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn load_agent_discovery_markers(conn: &Connection, sample_id: i64) -> Result<Vec<ScoredMarker>, String> {
    let sql = format!(
        "SELECT g.rsid, g.chromosome, g.allele1, g.allele2, c.clinical_significance,
                COALESCE(NULLIF(TRIM(d.gene), ''), NULLIF(TRIM(c.gene), ''), NULLIF(TRIM(e.gene), '')) AS gene
         FROM discovered_findings d
         JOIN genotypes g ON g.sample_id = d.sample_id AND g.rsid = d.rsid
         LEFT JOIN clinvar_reference c ON g.rsid = c.rsid
         LEFT JOIN ({EVIDENCE_GENE_SUBQUERY}) e ON g.rsid = e.rsid
         WHERE d.sample_id = ?"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id], |row| {
            Ok(row_to_scored_marker(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn load_gwas_discovery_markers(
    conn: &Connection,
    sample_id: i64,
    limit: u32,
) -> Result<Vec<ScoredMarker>, String> {
    let sql = format!(
        "SELECT g.rsid, g.chromosome, g.allele1, g.allele2, c.clinical_significance,
                COALESCE(NULLIF(TRIM(c.gene), ''), NULLIF(TRIM(e.gene), ''), NULLIF(TRIM(gw.primary_gene), '')) AS gene
         FROM genotypes g
         INNER JOIN gwas_reference gw ON g.rsid = gw.rsid
         LEFT JOIN clinvar_reference c ON g.rsid = c.rsid
         LEFT JOIN ({EVIDENCE_GENE_SUBQUERY}) e ON g.rsid = e.rsid
         WHERE g.sample_id = ?
         ORDER BY gw.association_count DESC, g.rsid
         LIMIT ?"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id, limit as i64], |row| {
            Ok(row_to_scored_marker(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn load_non_reference_markers(
    conn: &Connection,
    sample_id: i64,
    limit: u32,
) -> Result<Vec<ScoredMarker>, String> {
    let sql = format!(
        "SELECT g.rsid, g.chromosome, g.allele1, g.allele2, c.clinical_significance,
                COALESCE(NULLIF(TRIM(c.gene), ''), NULLIF(TRIM(e.gene), ''), NULLIF(TRIM(gw.primary_gene), '')) AS gene
         FROM genotypes g
         INNER JOIN gwas_reference gw ON g.rsid = gw.rsid
         LEFT JOIN clinvar_reference c ON g.rsid = c.rsid
         LEFT JOIN ({EVIDENCE_GENE_SUBQUERY}) e ON g.rsid = e.rsid
         WHERE g.sample_id = ?
           AND g.allele1 IN ('A','T','C','G')
           AND g.allele2 IN ('A','T','C','G')
           AND g.allele1 != g.allele2
         ORDER BY gw.association_count DESC, g.rsid
         LIMIT ?"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id, limit as i64], |row| {
            Ok(row_to_scored_marker(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn collect_markers_for_scopes(
    db_path: &Path,
    sample_id: i64,
    scope: &ResearchScopeConfig,
) -> Result<Vec<ScoredMarker>, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let mut merged: HashMap<String, ScoredMarker> = HashMap::new();

    if scope.curated {
        for marker in load_curated_markers(&conn, sample_id)? {
            merge_scored_marker(&mut merged, marker);
        }
    }
    if scope.agent_discoveries {
        for marker in load_agent_discovery_markers(&conn, sample_id)? {
            merge_scored_marker(&mut merged, marker);
        }
    }
    if scope.gwas_discovery {
        for marker in load_gwas_discovery_markers(&conn, sample_id, scope.gwas_discovery_limit)? {
            merge_scored_marker(&mut merged, marker);
        }
    }
    if scope.non_reference {
        for marker in load_non_reference_markers(&conn, sample_id, scope.non_reference_limit)? {
            merge_scored_marker(&mut merged, marker);
        }
    }

    let mut markers: Vec<ScoredMarker> = merged.into_values().collect();
    markers.sort_by(|a, b| {
        b.significance_score
            .partial_cmp(&a.significance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(markers)
}

pub fn preview_research_scope(
    db_path: &Path,
    sample_id: i64,
    scope: &ResearchScopeConfig,
) -> Result<ResearchScopePreview, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let genotype_total: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM genotypes WHERE sample_id = ?",
            params![sample_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as u64;
    let gwas_reference_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as u64;

    let gwas_genome_overlap: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM genotypes g INNER JOIN gwas_reference gw ON g.rsid = gw.rsid WHERE g.sample_id = ?",
            params![sample_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as u64;

    let curated = if scope.curated {
        load_curated_markers(&conn, sample_id)?.len() as u64
    } else {
        0
    };
    let agent_discoveries = if scope.agent_discoveries {
        load_agent_discovery_markers(&conn, sample_id)?.len() as u64
    } else {
        0
    };
    let gwas_discovery = if scope.gwas_discovery {
        load_gwas_discovery_markers(&conn, sample_id, scope.gwas_discovery_limit)?.len() as u64
    } else {
        0
    };
    let non_reference = if scope.non_reference {
        load_non_reference_markers(&conn, sample_id, scope.non_reference_limit)?.len() as u64
    } else {
        0
    };
    let total_unique = collect_markers_for_scopes(db_path, sample_id, scope)?.len() as u64;

    let gwas_beyond_cap = if scope.gwas_discovery {
        let cap = scope.gwas_discovery_limit as u64;
        gwas_genome_overlap.saturating_sub(cap.min(gwas_genome_overlap))
    } else {
        0
    };

    Ok(ResearchScopePreview {
        curated,
        agent_discoveries,
        gwas_discovery,
        non_reference,
        total_unique,
        genotype_total,
        gwas_reference_count,
        gwas_genome_overlap,
        gwas_beyond_cap,
    })
}

pub async fn get_all_sample_rsids(
    db_path: &Path,
    sample_id: i64,
) -> Result<Vec<ScoredMarker>, String> {
    collect_markers_for_scopes(db_path, sample_id, &ResearchScopeConfig {
        curated: true,
        agent_discoveries: false,
        gwas_discovery: false,
        non_reference: false,
        ..ResearchScopeConfig::default()
    })
}
