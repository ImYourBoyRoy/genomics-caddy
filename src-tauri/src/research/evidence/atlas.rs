// ./src-tauri/src/research/evidence/atlas.rs
//! 2D vector atlas — UMAP-style projection over sample embeddings.

use crate::research::types::QdrantConfig;
use crate::research::util::{string_to_u64, unix_now};
use crate::research::vector_store;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter};

const MAX_ATLAS_POINTS: usize = 2500;
const UMAP_NEIGHBORS: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasPoint {
    pub rsid: String,
    pub qdrant_point_id: Option<String>,
    pub x: f32,
    pub y: f32,
    pub trait_category: Option<String>,
    pub gene_symbol: Option<String>,
    pub data_quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorAtlasResult {
    pub sample_id: i64,
    pub point_count: u32,
    pub projection_method: String,
    pub points: Vec<AtlasPoint>,
    pub warning: String,
}

pub async fn build_vector_atlas(
    db_path: &Path,
    sample_id: i64,
    config: &QdrantConfig,
    limit: u32,
    app: Option<&AppHandle>,
) -> Result<VectorAtlasResult, String> {
    let cap = limit.min(MAX_ATLAS_POINTS as u32) as usize;
    let provider = vector_store::provider_from_config(config);
    emit_atlas_progress(
        app,
        "fetch",
        5,
        &format!("Fetching up to {cap} vectors from {}…", provider.as_str()),
    );
    let scrolled = vector_store::sample_vectors_for_atlas(config, sample_id, cap).await?;

    if scrolled.is_empty() {
        emit_atlas_progress(app, "done", 100, "No vectors to project");
        return Ok(VectorAtlasResult {
            sample_id,
            point_count: 0,
            projection_method: "none".into(),
            points: vec![],
            warning: format!(
                "No vectors found for this sample on {}. Run research enrichment first.",
                provider.as_str()
            ),
        });
    }

    emit_atlas_progress(
        app,
        "fetch",
        35,
        &format!("Loaded {} vectors — starting 2D projection…", scrolled.len()),
    );

    let vectors: Vec<Vec<f32>> = scrolled.iter().map(|p| p.vector.clone()).collect();
    let coords = project_umap_2d(&vectors, |epoch, total| {
        let pct = 35 + ((epoch as f32 / total.max(1) as f32) * 50.0) as u8;
        emit_atlas_progress(
            app,
            "project",
            pct.min(85),
            &format!("Projecting layout… epoch {epoch}/{total}"),
        );
    });

    emit_atlas_progress(app, "persist", 90, "Saving atlas points…");

    let points: Vec<AtlasPoint> = scrolled
        .iter()
        .zip(coords.iter())
        .map(|(item, (x, y))| AtlasPoint {
            rsid: item.rsid.clone(),
            qdrant_point_id: Some(item.point_id.clone()),
            x: *x,
            y: *y,
            trait_category: item.trait_category.clone(),
            gene_symbol: item.gene_symbol.clone(),
            data_quality_score: item.data_quality_score,
        })
        .collect();

    persist_atlas_points(db_path, sample_id, &points)?;
    emit_atlas_progress(
        app,
        "done",
        100,
        &format!("Atlas ready — {} points", points.len()),
    );

    Ok(VectorAtlasResult {
        sample_id,
        point_count: points.len() as u32,
        projection_method: format!("umap+{}", provider.as_str()),
        points,
        warning: format!(
            "Projected from {} index. Semantic proximity means related language/evidence context, not causality.",
            provider.as_str()
        ),
    })
}

fn emit_atlas_progress(app: Option<&AppHandle>, phase: &str, percent: u8, message: &str) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "vector:atlas_progress",
            serde_json::json!({
                "phase": phase,
                "percent": percent,
                "message": message,
            }),
        );
    }
}

pub fn load_atlas_points(
    conn: &Connection,
    sample_id: i64,
    limit: u32,
) -> Result<Vec<AtlasPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT rsid, qdrant_point_id, x, y, trait_category, gene_symbol, data_quality_score
             FROM vector_atlas_points WHERE sample_id = ? ORDER BY data_quality_score DESC LIMIT ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![sample_id, limit], |row| {
            Ok(AtlasPoint {
                rsid: row.get(0)?,
                qdrant_point_id: row.get(1)?,
                x: row.get::<_, f64>(2)? as f32,
                y: row.get::<_, f64>(3)? as f32,
                trait_category: row.get(4)?,
                gene_symbol: row.get(5)?,
                data_quality_score: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0) as f32,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn persist_atlas_points(
    db_path: &Path,
    sample_id: i64,
    points: &[AtlasPoint],
) -> Result<(), String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let now = unix_now();
    conn.execute(
        "DELETE FROM vector_atlas_points WHERE sample_id = ?",
        params![sample_id],
    )
    .map_err(|e| e.to_string())?;
    for p in points {
        let id = format!("atlas_{}_{}", sample_id, string_to_u64(&p.rsid));
        conn.execute(
            "INSERT INTO vector_atlas_points (
                atlas_id, sample_id, rsid, qdrant_point_id, x, y,
                trait_category, gene_symbol, data_quality_score, projection_method, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'umap', ?)",
            params![
                id,
                sample_id,
                p.rsid,
                p.qdrant_point_id,
                p.x,
                p.y,
                p.trait_category,
                p.gene_symbol,
                p.data_quality_score,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn project_umap_2d(
    vectors: &[Vec<f32>],
    mut on_epoch: impl FnMut(usize, usize),
) -> Vec<(f32, f32)> {
    let n = vectors.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(0.0, 0.0)];
    }
    if n == 2 {
        return vec![(-1.0, 0.0), (1.0, 0.0)];
    }

    let mut embedding: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let angle = (i as f64 / n as f64) * std::f64::consts::TAU;
            [angle.cos() * 5.0, angle.sin() * 5.0]
        })
        .collect();

    let k = UMAP_NEIGHBORS.min(n - 1).max(1);
    let mut neighbors: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for i in 0..n {
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (j, cosine_distance(&vectors[i], &vectors[j])))
            .collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        neighbors[i] = dists.into_iter().take(k).collect();
    }

    let epochs = 50usize;
    let lr = 1.0;
    let min_dist = 0.1f64;
    for epoch in 1..=epochs {
        for i in 0..n {
            let mut grad = [0.0f64; 2];
            for &(j, dist_high) in &neighbors[i] {
                let dx = embedding[i][0] - embedding[j][0];
                let dy = embedding[i][1] - embedding[j][1];
                let dist_low = (dx * dx + dy * dy).sqrt().max(1e-6);
                let q = (1.0 + (dist_low * dist_low) / min_dist).powi(-1);
                let coeff = if dist_high > 0.0 {
                    2.0 * 0.5 * (1.0 - q) / (dist_low * dist_high.sqrt())
                } else {
                    0.0
                };
                grad[0] += coeff * dx;
                grad[1] += coeff * dy;
            }
            embedding[i][0] -= lr * grad[0] / k as f64;
            embedding[i][1] -= lr * grad[1] / k as f64;
        }
        if epoch == 1 || epoch == epochs || epoch % 5 == 0 {
            on_epoch(epoch, epochs);
        }
    }

    normalize_2d(&embedding)
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    let len = a.len().min(b.len()).max(1);
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..len {
        dot += a[i] as f64 * b[i] as f64;
        na += a[i] as f64 * a[i] as f64;
        nb += b[i] as f64 * b[i] as f64;
    }
    let denom = (na.sqrt() * nb.sqrt()).max(1e-9);
    1.0 - (dot / denom).clamp(-1.0, 1.0)
}

fn normalize_2d(points: &[[f64; 2]]) -> Vec<(f32, f32)> {
    if points.is_empty() {
        return vec![];
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for p in points.iter() {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
    }
    let sx = (max_x - min_x).max(1e-6);
    let sy = (max_y - min_y).max(1e-6);
    points
        .iter()
        .map(|p| {
            (
                ((p[0] - min_x) / sx * 2.0 - 1.0) as f32,
                ((p[1] - min_y) / sy * 2.0 - 1.0) as f32,
            )
        })
        .collect()
}
