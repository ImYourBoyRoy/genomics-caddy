// ./src-tauri/src/offline/import_mane.rs
/*
Purpose: Import MANE Select summary TSV into the mane catalog DB.
*/

use super::compress::open_text_auto;
use rusqlite::{Connection, params};
use std::io::BufRead;
use std::path::Path;

pub fn import_mane_summary(conn: &Connection, path: &Path) -> Result<u64, String> {
    let reader = open_text_auto(path)?;
    let mut lines = reader.lines();

    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MANE summary empty".to_string())?;
    let headers: Vec<&str> = header.split('\t').collect();

    let gene_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("symbol") || h.contains("gene"))
        .unwrap_or(0);
    let ensembl_idx = headers.iter().position(|h| h.contains("Ensembl"));
    let refseq_idx = headers.iter().position(|h| h.contains("RefSeq"));
    let status_idx = headers.iter().position(|h| h.contains("MANE"));
    let coord_idx = headers
        .iter()
        .position(|h| h.contains("GRCh38") || h.contains("coordinates"));

    let table = if crate::offline::schema::schema_attached(conn, "mane") {
        "mane.mane_transcripts"
    } else {
        "reference.mane_transcripts"
    };
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    // Keep replacement and parsing in one transaction so a malformed stream
    // rolls back to the previous known-good catalog instead of leaving it empty.
    tx.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let gene = parts
            .get(gene_idx)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let Some(gene) = gene else { continue };

        let ensembl = ensembl_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let refseq = refseq_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let status = status_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let coords = coord_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());

        tx.execute(
            &format!(
                "INSERT OR REPLACE INTO {table}
                 (gene_symbol, ensembl_transcript, refseq_transcript, mane_status, grch38_coordinates)
                 VALUES (?, ?, ?, ?, ?)"
            ),
            params![gene, ensembl, refseq, status, coords],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_tsv_preserves_the_previous_catalog() {
        let data_dir =
            std::env::temp_dir().join(format!("dna_tools_mane_transaction_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).expect("create MANE transaction fixture directory");
        let db_path = data_dir.join("user_genome.db");
        let fixture_path = data_dir.join("mane-fixture.tsv");
        let conn = crate::db::connect(&db_path).expect("open MANE transaction fixture DB");
        crate::db::ensure_catalog_db_attached(&conn, &data_dir, "mane")
            .expect("attach MANE fixture catalog");
        conn.execute(
            "INSERT INTO mane.mane_transcripts
             (gene_symbol, ensembl_transcript, refseq_transcript, mane_status, grch38_coordinates)
             VALUES (?, ?, ?, ?, ?)",
            params![
                "OLDGENE",
                "ENST00000000001",
                "NM_000001",
                "MANE Select",
                "chr1:1-10"
            ],
        )
        .expect("seed previous MANE row");

        std::fs::write(
            &fixture_path,
            b"symbol\tEnsembl transcript\tRefSeq transcript\tMANE status\tGRCh38 coordinates\nNEWGENE\tENST00000000002\tNM_000002\tMANE Select\tchr2:1-10\n\xff\n",
        )
        .expect("write malformed MANE fixture");
        let error = import_mane_summary(&conn, &fixture_path)
            .expect_err("malformed MANE fixture should fail");
        assert!(!error.is_empty());

        let preserved_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mane.mane_transcripts", [], |row| {
                row.get(0)
            })
            .expect("count preserved MANE rows");
        assert_eq!(preserved_count, 1);
        let preserved_gene: String = conn
            .query_row("SELECT gene_symbol FROM mane.mane_transcripts", [], |row| {
                row.get(0)
            })
            .expect("read preserved MANE row");
        assert_eq!(preserved_gene, "OLDGENE");

        std::fs::write(
            &fixture_path,
            "symbol\tEnsembl transcript\tRefSeq transcript\tMANE status\tGRCh38 coordinates\nNEWGENE\tENST00000000002\tNM_000002\tMANE Select\tchr2:1-10\n",
        )
        .expect("write valid MANE retry fixture");
        assert_eq!(
            import_mane_summary(&conn, &fixture_path).expect("valid MANE retry should succeed"),
            1
        );
        let retried_gene: String = conn
            .query_row("SELECT gene_symbol FROM mane.mane_transcripts", [], |row| {
                row.get(0)
            })
            .expect("read retried MANE row");
        assert_eq!(retried_gene, "NEWGENE");
        drop(conn);
        std::fs::remove_dir_all(&data_dir).expect("remove MANE transaction fixture directory");
    }
}
