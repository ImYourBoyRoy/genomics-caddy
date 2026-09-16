// ./src-tauri/src/liftover.rs
/*
Module Docstring:
Purpose: Memory-efficient coordinate liftover (GRCh37 → GRCh38) using UCSC chain files.
Responsibilities:
- Parse `.chain` and `.chain.gz` files.
- Index mapping chains in memory by chromosome.
- Perform high-speed coordinate translation using binary search.
Key Inputs: Chain file path, source chromosome, and GRCh37 position.
Key Outputs: Option of mapped GRCh38 position.
Operational Notes: Implements UCSC liftover block mapping logic.
*/

use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone)]
struct ChainBlock {
    size: u64,
    dt: u64,
    dq: u64,
}

#[derive(Debug, Clone)]
struct InverseChainBlock {
    query_start: u64,
    query_end: u64,
    target_start: u64,
    query_strand: char,
}

#[derive(Debug, Clone)]
pub struct ChainRecord {
    pub t_name: String,
    pub t_start: u64,
    pub t_end: u64,
    pub q_name: String,
    pub q_size: u64,
    pub q_strand: char,
    pub q_start: u64,
    pub q_end: u64,
    blocks: Vec<ChainBlock>,
}

#[derive(Debug)]
pub struct LiftoverEngine {
    // Maps normalized chromosome (e.g., "1", "X") to sorted chain records
    chains: HashMap<String, Vec<ChainRecord>>,
    // Inverse index for direct GRCh38 source imports. Query coordinates are
    // stored in forward reference orientation even when qStrand is '-'.
    inverse_chains: HashMap<String, Vec<InverseChainBlock>>,
}

impl LiftoverEngine {
    /// Loads a chain file (uncompressed or .gz) and builds the liftover engine.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| format!("Failed to open chain file: {}", e))?;

        let reader: Box<dyn Read> = if path_ref
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gz"))
        {
            Box::new(GzDecoder::new(file))
        } else {
            Box::new(file)
        };

        let buf_reader = BufReader::new(reader);
        let mut chains: HashMap<String, Vec<ChainRecord>> = HashMap::new();
        let mut inverse_chains: HashMap<String, Vec<InverseChainBlock>> = HashMap::new();
        let mut current_record: Option<ChainRecord> = None;

        for line_res in buf_reader.lines() {
            let line = line_res.map_err(|e| format!("Failed to read line: {}", e))?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            if parts[0] == "chain" {
                // Save the previous record if any
                if let Some(rec) = current_record.take() {
                    let norm_chr = normalize_chr(&rec.t_name);
                    validate_chain_record(&rec)?;
                    add_inverse_blocks(&rec, &mut inverse_chains)?;
                    chains.entry(norm_chr).or_default().push(rec);
                }

                // Header line format:
                // chain score tName tSize tStrand tStart tEnd qName qSize qStrand qStart qEnd id
                if parts.len() < 13 {
                    return Err("Invalid chain header: expected at least 13 fields".to_string());
                }

                let t_name = parts[2].to_string();
                let t_start = parts[5]
                    .parse::<u64>()
                    .map_err(|_| "Invalid chain target start".to_string())?;
                let t_end = parts[6]
                    .parse::<u64>()
                    .map_err(|_| "Invalid chain target end".to_string())?;
                let q_name = parts[7].to_string();
                let q_size = parts[8]
                    .parse::<u64>()
                    .map_err(|_| "Invalid chain query size".to_string())?;
                let q_strand = parts[9]
                    .chars()
                    .next()
                    .filter(|strand| matches!(strand, '+' | '-'))
                    .ok_or_else(|| "Invalid chain query strand".to_string())?;
                let q_start = parts[10]
                    .parse::<u64>()
                    .map_err(|_| "Invalid chain query start".to_string())?;
                let q_end = parts[11]
                    .parse::<u64>()
                    .map_err(|_| "Invalid chain query end".to_string())?;

                current_record = Some(ChainRecord {
                    t_name,
                    t_start,
                    t_end,
                    q_name,
                    q_size,
                    q_strand,
                    q_start,
                    q_end,
                    blocks: Vec::new(),
                });
            } else if let Some(ref mut rec) = current_record {
                // Parse alignment block line
                if parts.len() == 3 {
                    let size = parts[0]
                        .parse::<u64>()
                        .map_err(|_| "Invalid chain block size".to_string())?;
                    let dt = parts[1]
                        .parse::<u64>()
                        .map_err(|_| "Invalid chain target gap".to_string())?;
                    let dq = parts[2]
                        .parse::<u64>()
                        .map_err(|_| "Invalid chain query gap".to_string())?;
                    rec.blocks.push(ChainBlock { size, dt, dq });
                } else if parts.len() == 1 {
                    let size = parts[0]
                        .parse::<u64>()
                        .map_err(|_| "Invalid final chain block size".to_string())?;
                    rec.blocks.push(ChainBlock { size, dt: 0, dq: 0 });
                } else {
                    return Err("Invalid chain alignment block".to_string());
                }
            }
        }

        // Push the final record
        if let Some(rec) = current_record {
            let norm_chr = normalize_chr(&rec.t_name);
            validate_chain_record(&rec)?;
            add_inverse_blocks(&rec, &mut inverse_chains)?;
            chains.entry(norm_chr).or_default().push(rec);
        }

        // Sort records by t_start for binary search
        for list in chains.values_mut() {
            list.sort_by_key(|r| r.t_start);
        }
        for list in inverse_chains.values_mut() {
            list.sort_by_key(|block| block.query_start);
        }

        Ok(LiftoverEngine {
            chains,
            inverse_chains,
        })
    }

    /// Translates a zero-based half-open GRCh37 coordinate to a zero-based
    /// GRCh38 coordinate. This mirrors UCSC chain coordinates exactly.
    pub fn liftover(&self, chromosome: &str, pos: u64) -> Option<u64> {
        let norm_chr = normalize_chr(chromosome);
        let records = self.chains.get(&norm_chr)?;

        // Binary search to find candidate records that overlap the position
        let idx = match records.binary_search_by_key(&pos, |r| r.t_start) {
            Ok(found) => found,
            Err(insert) => {
                if insert == 0 {
                    return None;
                }
                insert - 1
            }
        };

        let record = &records[idx];
        if pos < record.t_start || pos >= record.t_end {
            return None;
        }

        // Walk the alignment blocks within this chain
        let mut t_curr = record.t_start;
        let mut q_curr = if record.q_strand == '+' {
            record.q_start
        } else {
            record.q_size - record.q_end
        };

        for block in &record.blocks {
            if pos >= t_curr && pos < t_curr + block.size {
                let offset = pos - t_curr;
                if record.q_strand == '+' {
                    return Some(q_curr + offset);
                } else {
                    return Some(record.q_size - (q_curr + offset) - 1);
                }
            }
            t_curr += block.size + block.dt;
            q_curr += block.size + block.dq;
        }

        None
    }

    /// Translates the application's 1-based inclusive coordinate contract.
    pub fn liftover_1_based(&self, chromosome: &str, pos: u64) -> Option<u64> {
        let zero_based = pos.checked_sub(1)?;
        self.liftover(chromosome, zero_based)?.checked_add(1)
    }

    /// Translates a zero-based half-open GRCh38 coordinate to a zero-based
    /// GRCh37 coordinate using the inverse of the loaded UCSC chain. Query
    /// coordinates are normalized back to forward reference orientation.
    pub fn liftover_inverse(&self, chromosome: &str, pos: u64) -> Option<u64> {
        let norm_chr = normalize_chr(chromosome);
        let blocks = self.inverse_chains.get(&norm_chr)?;
        let idx = blocks
            .binary_search_by(|block| {
                if pos < block.query_start {
                    std::cmp::Ordering::Greater
                } else if pos >= block.query_end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()?;
        let block = &blocks[idx];
        let offset = if block.query_strand == '+' {
            pos.checked_sub(block.query_start)?
        } else {
            block.query_end.checked_sub(pos)?.checked_sub(1)?
        };
        block.target_start.checked_add(offset)
    }

    /// Translates the application's 1-based inclusive GRCh38 source
    /// coordinate contract back to a 1-based inclusive GRCh37 coordinate.
    pub fn liftover_inverse_1_based(&self, chromosome: &str, pos: u64) -> Option<u64> {
        let zero_based = pos.checked_sub(1)?;
        self.liftover_inverse(chromosome, zero_based)?.checked_add(1)
    }
}

fn add_inverse_blocks(
    record: &ChainRecord,
    inverse_chains: &mut HashMap<String, Vec<InverseChainBlock>>,
) -> Result<(), String> {
    let query_chr = normalize_chr(&record.q_name);
    let mut target_curr = record.t_start;
    let mut query_curr = if record.q_strand == '+' {
        record.q_start
    } else {
        record
            .q_size
            .checked_sub(record.q_end)
            .ok_or_else(|| "Chain query coordinate underflow".to_string())?
    };

    for block in &record.blocks {
        let target_end = target_curr
            .checked_add(block.size)
            .ok_or_else(|| "Chain target coordinate overflow".to_string())?;
        let query_end = query_curr
            .checked_add(block.size)
            .ok_or_else(|| "Chain query coordinate overflow".to_string())?;
        let (query_start_forward, query_end_forward) = if record.q_strand == '+' {
            (query_curr, query_end)
        } else {
            (
                record
                    .q_size
                    .checked_sub(query_end)
                    .ok_or_else(|| "Chain query coordinate underflow".to_string())?,
                record
                    .q_size
                    .checked_sub(query_curr)
                    .ok_or_else(|| "Chain query coordinate underflow".to_string())?,
            )
        };
        inverse_chains
            .entry(query_chr.clone())
            .or_default()
            .push(InverseChainBlock {
                query_start: query_start_forward,
                query_end: query_end_forward,
                target_start: target_curr,
                query_strand: record.q_strand,
            });

        target_curr = target_end
            .checked_add(block.dt)
            .ok_or_else(|| "Chain target gap overflow".to_string())?;
        query_curr = query_end
            .checked_add(block.dq)
            .ok_or_else(|| "Chain query gap overflow".to_string())?;
    }
    Ok(())
}

fn validate_chain_record(record: &ChainRecord) -> Result<(), String> {
    if record.t_start >= record.t_end
        || record.q_start > record.q_end
        || record.q_end > record.q_size
        || record.blocks.is_empty()
    {
        return Err("Invalid chain coordinate bounds".to_string());
    }
    let target_span = record
        .blocks
        .iter()
        .try_fold(0u64, |sum, block| sum.checked_add(block.size).and_then(|value| value.checked_add(block.dt)))
        .ok_or_else(|| "Chain target span overflow".to_string())?;
    let query_span = record
        .blocks
        .iter()
        .try_fold(0u64, |sum, block| sum.checked_add(block.size).and_then(|value| value.checked_add(block.dq)))
        .ok_or_else(|| "Chain query span overflow".to_string())?;
    if target_span != record.t_end - record.t_start
        || query_span != record.q_end - record.q_start
    {
        return Err("Chain block spans do not match chain header".to_string());
    }
    Ok(())
}

fn normalize_chr(chr: &str) -> String {
    let clean = chr.trim().to_ascii_uppercase();
    let clean = clean.strip_prefix("CHR").unwrap_or(&clean);
    match clean {
        "M" => "MT".to_string(),
        "23" => "X".to_string(),
        "24" => "Y".to_string(),
        "25" => "X".to_string(),
        "26" => "MT".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn engine(chain: &str) -> LiftoverEngine {
        let path = std::env::temp_dir().join(format!(
            "genomics_caddy_chain_test_{}_{}",
            std::process::id(),
            chain.len()
        ));
        let mut file = File::create(&path).expect("create chain fixture");
        file.write_all(chain.as_bytes()).expect("write chain fixture");
        let result = LiftoverEngine::new(&path);
        let _ = std::fs::remove_file(&path);
        result.expect("load chain fixture")
    }

    #[test]
    fn converts_one_based_input_to_zero_based_chain_and_back() {
        let chain = "chain 1 chr1 1000 + 100 125 chr1 1000 + 200 225 1\n10 5 5\n10\n";
        let liftover = engine(chain);
        assert_eq!(liftover.liftover_1_based("1", 106), Some(206));
        assert_eq!(liftover.liftover_1_based("1", 100), None);
        assert_eq!(liftover.liftover_1_based("1", 111), None);
    }

    #[test]
    fn handles_reverse_query_strand_with_one_based_contract() {
        let chain = "chain 1 chr1 1000 + 100 110 chr1 1000 - 700 710 2\n10\n";
        let liftover = engine(chain);
        assert_eq!(liftover.liftover_1_based("chr1", 101), Some(710));
        assert_eq!(liftover.liftover_inverse_1_based("chr1", 710), Some(101));
    }

    #[test]
    fn inverse_mapping_preserves_gaps_and_one_based_boundaries() {
        let chain = "chain 1 chr1 1000 + 100 125 chr1 1000 + 200 225 4\n10 5 5\n10\n";
        let liftover = engine(chain);
        assert_eq!(liftover.liftover_inverse_1_based("chr1", 206), Some(106));
        assert_eq!(liftover.liftover_inverse_1_based("chr1", 211), None);
        assert_eq!(liftover.liftover_inverse_1_based("chr1", 216), Some(116));
        assert_eq!(liftover.liftover_inverse_1_based("chr1", 200), None);
    }

    #[test]
    fn normalizes_ancestry_numeric_par_and_mitochondrial_aliases() {
        assert_eq!(normalize_chr("25"), "X");
        assert_eq!(normalize_chr("26"), "MT");
        assert_eq!(normalize_chr("chr26"), "MT");
    }

    #[test]
    fn rejects_inconsistent_chain_blocks() {
        let chain = "chain 1 chr1 1000 + 100 125 chr1 1000 + 200 225 3\n10 5 5\n9\n";
        let path = std::env::temp_dir().join(format!(
            "genomics_caddy_invalid_chain_{}_{}",
            std::process::id(),
            chain.len()
        ));
        std::fs::write(&path, chain).expect("write invalid chain");
        let error = LiftoverEngine::new(&path).expect_err("invalid chain should fail");
        let _ = std::fs::remove_file(path);
        assert!(error.contains("spans do not match"));
    }
}
