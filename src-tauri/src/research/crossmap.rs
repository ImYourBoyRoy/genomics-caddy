// ./src-tauri/src/research/crossmap.rs
/*
Purpose: Trait taxonomy, marker-pack bridging, and cross-reference tags for vector enrichment.
Maps GWAS traits to consultation modes, pack categories, and discovery catalog entries.
*/

use crate::db::get_pack_str;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct CrossMapContext {
    pub trait_categories: Vec<String>,
    pub consultation_modes: Vec<String>,
    pub pack_refs: Vec<serde_json::Value>,
    pub discovery_catalog_match: Option<serde_json::Value>,
    pub searchable_tags: Vec<String>,
    pub association_summary: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct ResearchTaxonomy {
    categories: Vec<TaxonomyCategory>,
}

#[derive(Debug, serde::Deserialize)]
struct TaxonomyCategory {
    id: String,
    keywords: Vec<String>,
    packs: Vec<String>,
    modes: Vec<String>,
}

fn load_taxonomy() -> Vec<TaxonomyCategory> {
    let Some(raw) = crate::db::get_support_resource_str("research_taxonomy") else {
        return Vec::new();
    };
    match serde_json::from_str::<ResearchTaxonomy>(&raw) {
        Ok(taxonomy) => taxonomy.categories,
        Err(error) => {
            eprintln!("Research taxonomy validation failed: {error}");
            Vec::new()
        }
    }
}

pub fn classify_traits(traits: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut categories = Vec::new();
    let mut packs = Vec::new();
    let mut modes = Vec::new();

    let taxonomy = load_taxonomy();
    for trait_name in traits {
        let lower = trait_name.to_lowercase();
        for category in &taxonomy {
            if category
                .keywords
                .iter()
                .any(|keyword| lower.contains(&keyword.to_lowercase()))
            {
                push_unique(&mut categories, category.id.clone());
                for pack in &category.packs {
                    push_unique(&mut packs, pack.clone());
                }
                for mode in &category.modes {
                    push_unique(&mut modes, mode.clone());
                }
            }
        }
    }

    (categories, packs, modes)
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !out.iter().any(|v| v == &value) {
        out.push(value);
    }
}

pub fn lookup_discovery_catalog(data_dir: Option<&Path>, rsid: &str) -> Option<serde_json::Value> {
    let catalog_str = get_pack_str(data_dir, "discovery_catalog")?;
    let catalog: serde_json::Value = serde_json::from_str(&catalog_str).ok()?;
    let markers = catalog["markers"].as_array()?;
    let rsid_lower = rsid.to_lowercase();
    markers.iter().find_map(|m| {
        let id = m["rsid"].as_str()?.to_lowercase();
        if id == rsid_lower {
            Some(serde_json::json!({
                "rsid": m["rsid"],
                "gene": m["gene"],
                "category": m["category"],
                "categories": m.get("categories").cloned().unwrap_or_else(|| serde_json::json!([])),
                "name": m["name"],
                "impact": m["impact"],
                "description": m["description"],
                "source": "discovery_catalog",
            }))
        } else {
            None
        }
    })
}

pub fn lookup_discovery_catalog_gene(data_dir: Option<&Path>, rsid: &str) -> Option<String> {
    lookup_discovery_catalog(data_dir, rsid)
        .and_then(|entry| entry["gene"].as_str().map(String::from))
        .filter(|g| {
            let n = g.trim();
            !n.is_empty() && !n.eq_ignore_ascii_case("unknown")
        })
}

pub fn lookup_marker_pack_refs(
    data_dir: Option<&Path>,
    rsid: &str,
    inferred_packs: &[String],
) -> Vec<serde_json::Value> {
    let mut refs = Vec::new();
    if let Some(catalog) = lookup_discovery_catalog(data_dir, rsid) {
        refs.push(catalog);
    }

    for pack_id in inferred_packs {
        refs.push(serde_json::json!({
            "pack_id": pack_id,
            "source": "trait_taxonomy",
            "note": "GWAS trait keywords mapped to marker pack for consultation cross-linking",
        }));
    }
    refs
}

pub fn build_searchable_tags(
    rsid: &str,
    gene: Option<&str>,
    traits: &[String],
    categories: &[String],
    packs: &[String],
) -> Vec<String> {
    let mut tags = vec![rsid.to_lowercase()];
    if let Some(g) = gene
        && !g.is_empty()
    {
        tags.push(g.to_lowercase());
    }
    for t in traits {
        let slug = t
            .to_lowercase()
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join("_");
        if !slug.is_empty() {
            tags.push(slug);
        }
    }
    for c in categories {
        tags.push(c.clone());
    }
    for p in packs {
        tags.push(format!("pack:{}", p));
    }
    tags.sort();
    tags.dedup();
    tags
}

pub fn build_association_summary(gwas_assocs: &[serde_json::Value]) -> serde_json::Value {
    let mut rows = Vec::new();
    for assoc in gwas_assocs.iter().take(10) {
        let trait_name = assoc["trait_name"]
            .as_str()
            .or_else(|| assoc["trait"]["trait"].as_str())
            .unwrap_or("");
        let pvalue = assoc["pvalue"].as_f64();
        let strength = pvalue.map(|p| {
            if p < 5e-8 {
                "genome_wide"
            } else if p < 1e-5 {
                "suggestive"
            } else {
                "weak"
            }
        });
        let mapped = assoc["mapped_gene"].as_str().filter(|s| !s.is_empty());
        let reported: Vec<String> = assoc["reported_genes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        rows.push(serde_json::json!({
            "trait": trait_name,
            "pvalue": pvalue,
            "strength": strength,
            "mapped_gene": mapped,
            "reported_genes": reported,
            "study_accession": assoc["study_accession"],
            "source": assoc["source"],
        }));
    }
    serde_json::json!({
        "count": gwas_assocs.len(),
        "top": rows,
    })
}

pub fn build_cross_map_context(
    data_dir: Option<&Path>,
    rsid: &str,
    gene: Option<&str>,
    traits: &[String],
    gwas_assocs: &[serde_json::Value],
) -> CrossMapContext {
    let (categories, pack_ids, modes) = classify_traits(traits);
    let catalog_match = lookup_discovery_catalog(data_dir, rsid);
    let pack_refs = lookup_marker_pack_refs(data_dir, rsid, &pack_ids);
    let searchable_tags = build_searchable_tags(rsid, gene, traits, &categories, &pack_ids);
    let association_summary = build_association_summary(gwas_assocs);

    CrossMapContext {
        trait_categories: categories,
        consultation_modes: modes,
        pack_refs,
        discovery_catalog_match: catalog_match,
        searchable_tags,
        association_summary,
    }
}

#[cfg(test)]
mod tests {
    use super::classify_traits;

    #[test]
    fn shared_taxonomy_routes_menstrual_and_prostate_traits() {
        let (categories, packs, modes) = classify_traits(&[
            "premenstrual mood symptoms".to_string(),
            "prostate cancer susceptibility".to_string(),
        ]);

        assert!(categories.contains(&"hormones_reproductive".to_string()));
        assert!(categories.contains(&"cancer_risk".to_string()));
        assert!(packs.contains(&"hormones_reproductive".to_string()));
        assert!(packs.contains(&"cancer_confirmation_only".to_string()));
        assert!(modes.contains(&"hormones_reproductive".to_string()));
    }

    #[test]
    fn shared_taxonomy_routes_other_existing_domains() {
        let (categories, packs, _) = classify_traits(&[
            "histamine hives".to_string(),
            "kidney stone".to_string(),
            "migraine headache".to_string(),
        ]);

        assert!(categories.contains(&"allergy_atopy_mast_cell".to_string()));
        assert!(categories.contains(&"kidney_fluid_electrolytes".to_string()));
        assert!(categories.contains(&"pain_migraine_sensory".to_string()));
        assert!(packs.contains(&"allergy_atopy_mast_cell".to_string()));
        assert!(packs.contains(&"kidney_fluid_electrolytes".to_string()));
        assert!(packs.contains(&"pain_migraine_sensory".to_string()));
    }
}
