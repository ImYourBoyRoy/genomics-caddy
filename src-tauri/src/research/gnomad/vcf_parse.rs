// ./src-tauri/src/research/gnomad/vcf_parse.rs
use super::types::{GnomadContext, GnomadLookupStatus, GnomadSourceMode};
#[derive(Debug, Clone)]
pub struct ParsedSiteRecord {
    pub chrom: String,
    pub pos: i64,
    pub ref_allele: String,
    pub alt_alleles: Vec<String>,
    pub id_field: Vec<String>,
    pub filters: Vec<String>,
    pub ac: Option<i64>,
    pub an: Option<i64>,
    pub af: Option<f64>,
    pub popmax: Option<f64>,
    pub popmax_population: Option<String>,
    pub hom_count: Option<i64>,
}

pub fn normalize_allele(a: &str) -> String {
    a.trim().to_uppercase()
}

pub fn match_user_allele(
    ref_allele: &str,
    alt_alleles: &[String],
    allele1: &str,
    allele2: &str,
) -> Option<(String, String)> {
    let ref_n = normalize_allele(ref_allele);
    let alts: Vec<String> = alt_alleles.iter().map(|a| normalize_allele(a)).collect();
    let a1 = normalize_allele(allele1);
    let a2 = normalize_allele(allele2);
    if a1.is_empty() || a2.is_empty() {
        return None;
    }
    let user = [a1, a2];
    for alt in &alts {
        let valid = |a: &str| a == ref_n || alts.contains(&a.to_string());
        if valid(&user[0]) && valid(&user[1]) {
            if user[0] == ref_n && user[1] == ref_n {
                return Some((ref_n.clone(), ref_n.clone()));
            }
            let user_alt = if user[0] != ref_n {
                user[0].clone()
            } else {
                user[1].clone()
            };
            if alts.contains(&user_alt) {
                return Some((ref_n.clone(), user_alt));
            }
        }
        let _ = alt;
    }
    None
}

pub fn parse_site_line(line: &str) -> Option<ParsedSiteRecord> {
    if line.starts_with('#') {
        return None;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 8 {
        return None;
    }
    let pos = cols[1].parse::<i64>().ok()?;
    let ref_allele = cols[3].to_string();
    let alt_alleles: Vec<String> = cols[4]
        .split(',')
        .filter(|a| !a.is_empty() && *a != ".")
        .map(String::from)
        .collect();
    let id_field: Vec<String> = cols[2]
        .split(';')
        .filter(|id| !id.is_empty() && *id != ".")
        .map(String::from)
        .collect();
    let filters: Vec<String> = if cols[6] == "." {
        Vec::new()
    } else {
        cols[6].split(';').map(String::from).collect()
    };

    let info = parse_info_map(cols[7]);
    let ac = parse_int_array_first(&info, "AC").or_else(|| parse_int(&info, "AC"));
    let an = parse_int(&info, "AN");
    let af = parse_float_array_first(&info, "AF").or_else(|| parse_float_from_map(&info, "AF"));
    let popmax = parse_float_from_map(&info, "popmax");
    let popmax_population = info.get("popmax_population").cloned();
    let hom_count = parse_int_array_first(&info, "nhomalt").or_else(|| parse_int(&info, "nhomalt"));

    Some(ParsedSiteRecord {
        chrom: cols[0].trim_start_matches("chr").to_string(),
        pos,
        ref_allele,
        alt_alleles,
        id_field,
        filters,
        ac,
        an,
        af,
        popmax,
        popmax_population,
        hom_count,
    })
}

pub fn record_to_context(
    rec: &ParsedSiteRecord,
    alt: &str,
    release: &str,
    dataset: &str,
    source_mode: GnomadSourceMode,
    source_url: &str,
    status: GnomadLookupStatus,
    user_match: Option<&str>,
) -> GnomadContext {
    GnomadContext {
        lookup_status: status.as_str().to_string(),
        release: release.to_string(),
        dataset: dataset.to_string(),
        variant_id: Some(format!("{}-{}-{}-{}", rec.chrom, rec.pos, rec.ref_allele, alt)),
        rsids: rec
            .id_field
            .iter()
            .filter(|id| id.starts_with("rs"))
            .cloned()
            .collect(),
        ac: rec.ac,
        an: rec.an,
        af: rec.af,
        ac_exomes: if dataset == "exomes" { rec.ac } else { None },
        an_exomes: if dataset == "exomes" { rec.an } else { None },
        af_exomes: if dataset == "exomes" { rec.af } else { None },
        ac_genomes: if dataset == "genomes" { rec.ac } else { None },
        an_genomes: if dataset == "genomes" { rec.an } else { None },
        af_genomes: if dataset == "genomes" { rec.af } else { None },
        popmax: rec.popmax,
        popmax_population: rec.popmax_population.clone(),
        faf95_popmax: None,
        faf95_popmax_population: None,
        homozygote_count: rec.hom_count,
        hemizygote_count: None,
        filters: rec.filters.clone(),
        flags: Vec::new(),
        population_frequencies: serde_json::json!({}),
        source_url: Some(source_url.to_string()),
        source_mode: source_mode.as_str().to_string(),
        fetched_at: Some(unix_now()),
        warnings: vec![
            "Population frequency context only — not clinical significance.".into(),
        ],
        user_allele_match_status: user_match.map(String::from),
    }
}

fn parse_info_map(info: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for part in info.split(';') {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        } else if !part.is_empty() && part != "." {
            map.insert(part.to_string(), "1".into());
        }
    }
    map
}

fn parse_int(map: &std::collections::HashMap<String, String>, key: &str) -> Option<i64> {
    map.get(key).and_then(|v| v.parse().ok())
}

fn parse_int_array_first(map: &std::collections::HashMap<String, String>, key: &str) -> Option<i64> {
    map.get(key)
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.parse().ok())
}

fn parse_float_from_map(map: &std::collections::HashMap<String, String>, key: &str) -> Option<f64> {
    map.get(key)
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.parse().ok())
}

fn parse_float_array_first(map: &std::collections::HashMap<String, String>, key: &str) -> Option<f64> {
    parse_float_from_map(map, key)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
