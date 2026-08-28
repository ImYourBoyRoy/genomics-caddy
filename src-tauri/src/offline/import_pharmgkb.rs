// ./src-tauri/src/offline/import_pharmgkb.rs
use crate::research::util::normalize_rsid;
use rusqlite::{Connection, params};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

fn extract_tsv_from_zip(zip_path: &Path, needle: &str) -> Result<Vec<u8>, String> {
    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.to_lowercase();
        if !name.ends_with(".tsv") && !name.ends_with(".txt") {
            continue;
        }
        if !name.contains(needle) && needle != "any" {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        return Ok(buf);
    }
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.to_lowercase();
        if name.ends_with(".tsv") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            return Ok(buf);
        }
    }
    Err(format!("No TSV found in {}", zip_path.display()))
}

fn col_index(headers: &[&str], name: &str) -> Option<usize> {
    headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(name) || h.replace(' ', "") == name.replace(' ', ""))
}

pub fn import_pharmgkb_clinical_variants(
    conn: &Connection,
    zip_path: &Path,
) -> Result<u64, String> {
    let bytes = extract_tsv_from_zip(zip_path, "clinical")?;
    let table = if crate::offline::schema::schema_attached(conn, "pharmgkb") {
        "pharmgkb.pharmgkb_clinical_variants"
    } else {
        "reference.pharmgkb_clinical_variants"
    };
    let reader = BufReader::new(bytes.as_slice());
    let mut lines = reader.lines();
    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "PharmGKB clinical TSV empty".to_string())?;
    let headers: Vec<&str> = header.split('\t').collect();

    let rs_idx = col_index(&headers, "Variant/Haplotypes")
        .or_else(|| col_index(&headers, "RSID"))
        .or_else(|| col_index(&headers, "Variant"));
    let gene_idx = col_index(&headers, "Gene");
    let drug_idx = col_index(&headers, "Drug(s)").or_else(|| col_index(&headers, "Chemical"));
    let pheno_idx =
        col_index(&headers, "Phenotype Category").or_else(|| col_index(&headers, "Phenotype"));
    let level_idx =
        col_index(&headers, "Level of Evidence").or_else(|| col_index(&headers, "Evidence Level"));

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    // Keep replacement and parsing in one transaction so a malformed stream
    // rolls back to the previous known-good catalog instead of leaving it empty.
    tx.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let rsid = rs_idx
            .and_then(|i| parts.get(i))
            .and_then(|s| normalize_rsid(s.trim()))
            .or_else(|| {
                rs_idx
                    .and_then(|i| parts.get(i))
                    .and_then(|s| s.split(';').find_map(|t| normalize_rsid(t.trim())))
            });
        let Some(rsid) = rsid else {
            continue;
        };
        let gene = gene_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let drug = drug_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let phenotype = pheno_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let evidence = level_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        tx.execute(
            &format!(
                "INSERT INTO {table} (rsid, gene, drug, phenotype, evidence_level, raw_json)
                 VALUES (?, ?, ?, ?, ?, ?)"
            ),
            params![rsid, gene, drug, phenotype, evidence, line],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

pub fn import_pharmgkb_genes(conn: &Connection, zip_path: &Path) -> Result<u64, String> {
    let bytes = extract_tsv_from_zip(zip_path, "genes")?;
    let table = if crate::offline::schema::schema_attached(conn, "pharmgkb") {
        "pharmgkb.pharmgkb_genes"
    } else {
        "reference.pharmgkb_genes"
    };
    let reader = BufReader::new(bytes.as_slice());
    let mut lines = reader.lines();
    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "PharmGKB genes TSV empty".to_string())?;
    let headers: Vec<&str> = header.split('\t').collect();
    let id_idx = col_index(&headers, "PharmGKB Accession Id")
        .or_else(|| col_index(&headers, "Accession Id"));
    let sym_idx = col_index(&headers, "Symbol").or_else(|| col_index(&headers, "Gene Symbol"));
    let name_idx = col_index(&headers, "Name");

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    // Keep replacement and parsing in one transaction so a malformed stream
    // rolls back to the previous known-good catalog instead of leaving it empty.
    tx.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    let mut count = 0u64;
    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let id = id_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let symbol = sym_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let Some(symbol) = symbol else { continue };
        let name = name_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let pgx_id = id.unwrap_or_else(|| symbol.clone());
        tx.execute(
            &format!(
                "INSERT OR REPLACE INTO {table} (pharmgkb_id, symbol, name, raw_json) VALUES (?, ?, ?, ?)"
            ),
            params![pgx_id, symbol, name, line],
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
    use std::io::Write;
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn write_zip(path: &Path, entry_name: &str, contents: &[u8]) {
        let file = File::create(path).expect("create PharmGKB zip fixture");
        let mut archive = ZipWriter::new(file);
        archive
            .start_file(entry_name, SimpleFileOptions::default())
            .expect("create PharmGKB zip entry");
        archive
            .write_all(contents)
            .expect("write PharmGKB zip entry");
        archive.finish().expect("finish PharmGKB zip fixture");
    }

    #[test]
    fn malformed_clinical_tsv_preserves_the_previous_catalog() {
        let data_dir = std::env::temp_dir().join(format!(
            "dna_tools_pharmgkb_clinical_transaction_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir)
            .expect("create PharmGKB clinical transaction fixture directory");
        let db_path = data_dir.join("user_genome.db");
        let fixture_path = data_dir.join("clinicalVariants.zip");
        let conn = crate::db::connect(&db_path).expect("open PharmGKB transaction fixture DB");
        crate::db::ensure_catalog_db_attached(&conn, &data_dir, "pharmgkb")
            .expect("attach PharmGKB fixture catalog");
        conn.execute(
            "INSERT INTO pharmgkb.pharmgkb_clinical_variants
             (rsid, gene, drug, phenotype, evidence_level, raw_json)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                "rs-old",
                "OLDGENE",
                "OldDrug",
                "Old fixture phenotype",
                "4",
                "fixture"
            ],
        )
        .expect("seed previous PharmGKB clinical row");

        write_zip(
            &fixture_path,
            "clinicalVariants.tsv",
            b"Variant/Haplotypes\tGene\tDrug(s)\nrs123\tNEWGENE\tNewDrug\n\xff\n",
        );
        let error = import_pharmgkb_clinical_variants(&conn, &fixture_path)
            .expect_err("malformed PharmGKB clinical fixture should fail");
        assert!(!error.is_empty());

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pharmgkb.pharmgkb_clinical_variants",
                [],
                |row| row.get(0),
            )
            .expect("count preserved PharmGKB clinical rows");
        assert_eq!(count, 1);
        assert_eq!(
            conn.query_row(
                "SELECT rsid FROM pharmgkb.pharmgkb_clinical_variants",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read preserved PharmGKB clinical row"),
            "rs-old"
        );

        write_zip(
            &fixture_path,
            "clinicalVariants.tsv",
            b"Variant/Haplotypes\tGene\tDrug(s)\nrs123\tNEWGENE\tNewDrug\n",
        );
        assert_eq!(
            import_pharmgkb_clinical_variants(&conn, &fixture_path)
                .expect("valid PharmGKB clinical retry should succeed"),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT rsid FROM pharmgkb.pharmgkb_clinical_variants",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read retried PharmGKB clinical row"),
            "rs123"
        );
        drop(conn);
        std::fs::remove_dir_all(&data_dir)
            .expect("remove PharmGKB clinical transaction fixture directory");
    }

    #[test]
    fn malformed_genes_tsv_preserves_the_previous_catalog() {
        let data_dir = std::env::temp_dir().join(format!(
            "dna_tools_pharmgkb_genes_transaction_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir)
            .expect("create PharmGKB genes transaction fixture directory");
        let db_path = data_dir.join("user_genome.db");
        let fixture_path = data_dir.join("genes.zip");
        let conn = crate::db::connect(&db_path).expect("open PharmGKB genes fixture DB");
        crate::db::ensure_catalog_db_attached(&conn, &data_dir, "pharmgkb")
            .expect("attach PharmGKB genes fixture catalog");
        conn.execute(
            "INSERT INTO pharmgkb.pharmgkb_genes
             (pharmgkb_id, symbol, name, raw_json) VALUES (?, ?, ?, ?)",
            params!["PA0001", "OLDGENE", "Old fixture gene", "fixture"],
        )
        .expect("seed previous PharmGKB genes row");

        write_zip(
            &fixture_path,
            "genes.tsv",
            b"PharmGKB Accession Id\tSymbol\tName\nPA0002\tNEWGENE\tNew fixture gene\n\xff\n",
        );
        let error = import_pharmgkb_genes(&conn, &fixture_path)
            .expect_err("malformed PharmGKB genes fixture should fail");
        assert!(!error.is_empty());

        let preserved_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pharmgkb.pharmgkb_genes", [], |row| {
                row.get(0)
            })
            .expect("count preserved PharmGKB genes rows");
        assert_eq!(preserved_count, 1);
        let preserved_symbol: String = conn
            .query_row("SELECT symbol FROM pharmgkb.pharmgkb_genes", [], |row| {
                row.get(0)
            })
            .expect("read preserved PharmGKB genes row");
        assert_eq!(preserved_symbol, "OLDGENE");

        write_zip(
            &fixture_path,
            "genes.tsv",
            b"PharmGKB Accession Id\tSymbol\tName\nPA0002\tNEWGENE\tNew fixture gene\n",
        );
        assert_eq!(
            import_pharmgkb_genes(&conn, &fixture_path)
                .expect("valid PharmGKB genes retry should succeed"),
            1
        );
        let retried_symbol: String = conn
            .query_row("SELECT symbol FROM pharmgkb.pharmgkb_genes", [], |row| {
                row.get(0)
            })
            .expect("read retried PharmGKB genes row");
        assert_eq!(retried_symbol, "NEWGENE");
        drop(conn);
        std::fs::remove_dir_all(&data_dir)
            .expect("remove PharmGKB genes transaction fixture directory");
    }
}
