// ./src-tauri/src/research/gnomad/allele_norm.rs
//! Normalize chromosomes and alleles for cache keys and gnomAD matching.

pub fn normalize_chrom_key(chrom: &str) -> String {
    let c = chrom.trim().trim_start_matches("chr").to_uppercase();
    if c == "MT" { "M".to_string() } else { c }
}

pub fn normalize_allele_token(a: &str) -> String {
    a.trim().to_uppercase()
}

/// Left-align SNVs/indels for stable cache keys (basic normalization).
pub fn normalize_ref_alt_pair(ref_allele: &str, alt_allele: &str) -> (String, String) {
    let mut reference = normalize_allele_token(ref_allele);
    let mut alternate = normalize_allele_token(alt_allele);
    if reference.is_empty() || alternate.is_empty() {
        return (reference, alternate);
    }
    if reference == alternate {
        return (reference, alternate);
    }
    // SNV — already stable
    if reference.len() == 1 && alternate.len() == 1 {
        return (reference, alternate);
    }
    // Left-trim shared prefix
    while !reference.is_empty()
        && !alternate.is_empty()
        && reference.as_bytes()[0] == alternate.as_bytes()[0]
    {
        reference.remove(0);
        alternate.remove(0);
    }
    // Left-pad with first base from original ref when indel
    if reference.is_empty() || alternate.is_empty() {
        let orig_ref = normalize_allele_token(ref_allele);
        if let Some(first) = orig_ref.chars().next() {
            if reference.is_empty() {
                reference.push(first);
            }
            if alternate.is_empty() {
                alternate.push(first);
            }
        }
    }
    (reference, alternate)
}

pub fn cache_alleles(ref_allele: &str, alt_allele: &str) -> (String, String) {
    normalize_ref_alt_pair(ref_allele, alt_allele)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snv_uppercase() {
        let (r, a) = cache_alleles("a", "g");
        assert_eq!(r, "A");
        assert_eq!(a, "G");
    }

    #[test]
    fn chrom_normalization() {
        assert_eq!(normalize_chrom_key("chr22"), "22");
        assert_eq!(normalize_chrom_key("MT"), "M");
    }
}
