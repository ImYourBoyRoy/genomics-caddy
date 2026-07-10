// ./src-tauri/src/research/evidence/catalog.rs
//! Paginated, filter-only catalog browse over association_facts (no semantic search).

use super::card::evidence_card_from_payload;
use super::types::{BrowseAssociationsParams, BrowseAssociationsResult};
use crate::research::qdrant::find_point_payload_by_rsid;
use crate::research::types::QdrantConfig;
use rusqlite::{Connection, types::Value};
use std::path::Path;

fn order_clause(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("wellness") {
        "clinical" => "cs DESC, ws DESC, dq DESC, rsid ASC",
        "dq" => "dq DESC, ws DESC, rsid ASC",
        "rsid" => "rsid ASC",
        "gene" => "gene ASC, rsid ASC",
        _ => "ws DESC, cs DESC, dq DESC, rsid ASC",
    }
}

fn score_having(preset: &str) -> Option<(&'static str, f64)> {
    match preset {
        "clinical" => Some(("cs >= ?", 0.25)),
        "actionable" => Some(("ws >= ?", 0.25)),
        _ => None,
    }
}

fn build_catalog_sql(
    p: &BrowseAssociationsParams,
    count_only: bool,
) -> Result<(String, Vec<Value>), String> {
    let min_dq = p.min_data_quality.unwrap_or(0.0) as f64;
    let mut binds: Vec<Value> = vec![Value::Integer(p.sample_id)];
    let mut where_parts = vec!["sample_id = ?1".to_string()];
    let mut n: i32 = 2;

    if let Some(ref cat) = p.trait_category {
        where_parts.push(format!("trait_category = ?{n}"));
        binds.push(Value::Text(cat.clone()));
        n += 1;
    }
    if min_dq > 0.0 {
        where_parts.push(format!("data_quality_score >= ?{n}"));
        binds.push(Value::Real(min_dq));
        n += 1;
    }
    if p.preset == "gwas" {
        where_parts.push("association_type = 'gwas_top_association'".into());
    }
    if p.preset == "unknown" {
        where_parts.push(
            "(personal_direction IS NULL OR personal_direction IN ('unknown','possible_relevance_unknown_direction'))".into(),
        );
    }
    if let Some(ref dir) = p.direction_filter {
        if dir == "known" {
            where_parts.push(
                "personal_direction IS NOT NULL AND personal_direction NOT IN ('unknown','possible_relevance_unknown_direction')".into(),
            );
        } else if dir == "unknown" {
            where_parts.push(
                "(personal_direction IS NULL OR personal_direction IN ('unknown','possible_relevance_unknown_direction'))".into(),
            );
        }
    }
    if let Some(ref text) = p.text_filter {
        let t = text.trim();
        if !t.is_empty() {
            let rsid_pat = if t.to_lowercase().starts_with("rs") {
                format!("{}%", t.to_lowercase())
            } else {
                format!("%{}%", t.to_lowercase())
            };
            where_parts.push(format!(
                "(LOWER(rsid) LIKE ?{n} OR mapped_gene_symbol LIKE ?{})",
                n + 1
            ));
            binds.push(Value::Text(rsid_pat));
            binds.push(Value::Text(format!("%{}%", t)));
            n += 2;
        }
    }

    let where_sql = where_parts.join(" AND ");
    let having = score_having(&p.preset);
    let having_sql = having
        .map(|(clause, _)| format!(" HAVING {clause}"))
        .unwrap_or_default();

    if count_only {
        if let Some((_, floor)) = having {
            binds.push(Value::Real(floor));
        }
        let sql = format!(
            "SELECT COUNT(*) FROM (
                SELECT rsid,
                  MAX(wellness_actionability_score) AS ws,
                  MAX(clinical_actionability_score) AS cs
                FROM association_facts
                WHERE {where_sql}
                GROUP BY rsid{having_sql}
             )"
        );
        return Ok((sql, binds));
    }

    if let Some((_, floor)) = having {
        binds.push(Value::Real(floor));
    }
    let limit_idx = n;
    binds.push(Value::Integer(p.limit.clamp(5, 100) as i64));
    n += 1;
    let offset_idx = n;
    binds.push(Value::Integer(p.offset as i64));

    let order = order_clause(p.sort.as_deref());
    let sql = format!(
        "SELECT rsid FROM (
            SELECT rsid,
              MAX(wellness_actionability_score) AS ws,
              MAX(clinical_actionability_score) AS cs,
              MAX(data_quality_score) AS dq,
              MAX(mapped_gene_symbol) AS gene
            FROM association_facts
            WHERE {where_sql}
            GROUP BY rsid{having_sql}
            ORDER BY {order}
            LIMIT ?{limit_idx} OFFSET ?{offset_idx}
         )"
    );
    Ok((sql, binds))
}

pub fn list_catalog_rsids(
    conn: &Connection,
    p: &BrowseAssociationsParams,
) -> Result<Vec<String>, String> {
    let (sql, binds) = build_catalog_sql(p, false)?;
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    let rows = stmt
        .query_map(refs.as_slice(), |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn count_catalog_rsids(conn: &Connection, p: &BrowseAssociationsParams) -> Result<u64, String> {
    let (sql, binds) = build_catalog_sql(p, true)?;
    let refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    conn.query_row(&sql, refs.as_slice(), |row| row.get::<_, i64>(0))
        .map(|n| n.max(0) as u64)
        .map_err(|e| e.to_string())
}

pub async fn browse_catalog(
    params: &BrowseAssociationsParams,
    config: &QdrantConfig,
    db_path: &Path,
) -> Result<BrowseAssociationsResult, String> {
    let sample_id = params.sample_id;
    let limit = params.limit.clamp(5, 100);
    let offset = params.offset;
    let params_owned = params.clone();

    let (rsids, total_count) = {
        let db_path = db_path.to_path_buf();
        tauri::async_runtime::spawn_blocking(move || {
            let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
            let total = count_catalog_rsids(&conn, &params_owned)?;
            let rsids = list_catalog_rsids(&conn, &params_owned)?;
            Ok::<_, String>((rsids, total))
        })
        .await
        .map_err(|e| format!("Catalog browse worker failed: {}", e))??
    };

    let mut cards = Vec::new();
    for rsid in rsids {
        if let Some((payload, point_id)) = find_point_payload_by_rsid(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            sample_id,
            &rsid,
        )
        .await?
        {
            let score = payload["wellness_actionability_score"]
                .as_f64()
                .or_else(|| payload["clinical_actionability_score"].as_f64())
                .or_else(|| payload["data_quality_score"].as_f64())
                .unwrap_or(0.5) as f32;
            cards.push(evidence_card_from_payload(
                &payload,
                score,
                None,
                Some(point_id),
            ));
        }
    }

    Ok(BrowseAssociationsResult {
        cards,
        total_count,
        offset,
        limit,
    })
}
