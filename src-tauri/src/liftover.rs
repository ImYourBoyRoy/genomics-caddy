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

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use flate2::read::GzDecoder;

#[derive(Debug, Clone)]
struct ChainBlock {
    size: u64,
    dt: u64,
    dq: u64,
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

pub struct LiftoverEngine {
    // Maps normalized chromosome (e.g., "1", "X") to sorted chain records
    chains: HashMap<String, Vec<ChainRecord>>,
}

impl LiftoverEngine {
    /// Loads a chain file (uncompressed or .gz) and builds the liftover engine.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| format!("Failed to open chain file: {}", e))?;
        
        let reader: Box<dyn Read> = if path_ref.extension().is_some_and(|ext| ext == "gz") {
            Box::new(GzDecoder::new(file))
        } else {
            Box::new(file)
        };

        let buf_reader = BufReader::new(reader);
        let mut chains: HashMap<String, Vec<ChainRecord>> = HashMap::new();
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
                    chains.entry(norm_chr).or_default().push(rec);
                }

                // Header line format:
                // chain score tName tSize tStrand tStart tEnd qName qSize qStrand qStart qEnd id
                if parts.len() < 12 {
                    continue;
                }

                let t_name = parts[2].to_string();
                let t_start = parts[5].parse::<u64>().unwrap_or(0);
                let t_end = parts[6].parse::<u64>().unwrap_or(0);
                let q_name = parts[7].to_string();
                let q_size = parts[8].parse::<u64>().unwrap_or(0);
                let q_strand = parts[9].chars().next().unwrap_or('+');
                let q_start = parts[10].parse::<u64>().unwrap_or(0);
                let q_end = parts[11].parse::<u64>().unwrap_or(0);

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
                    let size = parts[0].parse::<u64>().unwrap_or(0);
                    let dt = parts[1].parse::<u64>().unwrap_or(0);
                    let dq = parts[2].parse::<u64>().unwrap_or(0);
                    rec.blocks.push(ChainBlock { size, dt, dq });
                } else if parts.len() == 1 {
                    let size = parts[0].parse::<u64>().unwrap_or(0);
                    rec.blocks.push(ChainBlock { size, dt: 0, dq: 0 });
                }
            }
        }

        // Push the final record
        if let Some(rec) = current_record {
            let norm_chr = normalize_chr(&rec.t_name);
            chains.entry(norm_chr).or_default().push(rec);
        }

        // Sort records by t_start for binary search
        for list in chains.values_mut() {
            list.sort_by_key(|r| r.t_start);
        }

        Ok(LiftoverEngine { chains })
    }

    /// Translates a GRCh37 coordinate to GRCh38.
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
}

fn normalize_chr(chr: &str) -> String {
    chr.replace("chr", "").replace("Chr", "").trim().to_string()
}
