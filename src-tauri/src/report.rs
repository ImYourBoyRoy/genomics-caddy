// ./src-tauri/src/report.rs
/*
Module Docstring:
Purpose: Direction-aware, template-based report generator for genetic trait profiling.
Responsibilities:
- Evaluate effect alleles for custom marker profiles defined in JSON.
- Classify each marker's severity based on effect_direction, evidence_tier, effect_count,
  and clinical_confirmation_required — not just raw allele count.
- Compute direction-aware section summaries with separate tallies for risk, protective,
  trait, and context-dependent markers.
- Only risk-direction markers contribute to percent signal scores.
- Sections where all markers require clinical confirmation suppress percent display.
Key Inputs: SQLite connection, sample ID, and report template JSON.
Key Outputs: Generated report JSON containing evaluated markers, severity classes, and section summaries.
Operational Notes: Designed for consumer-grade raw DNA, not clinical diagnostics.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rusqlite::Connection;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectDirection {
    Risk,
    Protective,
    ContextDependent,
    Trait,
    Unknown,
}

// ---------------------------------------------------------------------------
// Input structs (deserialized from JSON template)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerSource {
    pub name: String,
    pub url: Option<String>,
    pub accessed: Option<String>,
    pub evidence_type: Option<String>,
    pub conflict_of_interest: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MarkerDefinition {
    pub rsid: String,
    pub gene: String,
    pub variant_name: Option<String>,
    #[serde(alias = "risk_allele")]
    pub effect_allele: String,
    pub impact: String,
    pub evidence_tier: String,
    pub interpretation: String,
    pub do_not_claim: Vec<String>,
    pub confirm_with: Vec<String>,
    pub effect_direction: EffectDirection,
    pub raw_dna_limitation: Option<String>,
    pub clinical_confirmation_required: Option<bool>,
    pub sources: Option<Vec<MarkerSource>>,
    pub variant_type: Option<String>,
    pub expected_plus_alleles: Option<Vec<String>>,
    pub strand: Option<String>,
    pub source_build: Option<String>,
    pub hgvs: Option<String>,
    pub allele_orientation_verified: Option<bool>,
    pub orientation_source: Option<String>,
    pub interpretation_blocked_if_unverified: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SectionDefinition {
    pub name: String,
    pub markers: Vec<MarkerDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ReportTemplate {
    pub title: String,
    pub description: String,
    pub sections: Vec<SectionDefinition>,
}

// ---------------------------------------------------------------------------
// Output structs (serialized to JSON for the frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EvaluatedMarker {
    pub rsid: String,
    pub gene: String,
    pub variant_name: Option<String>,
    pub user_genotype: String,
    pub effect_allele: String,
    pub effect_count: u8,
    pub impact: String,
    pub evidence_tier: String,
    pub interpretation: String,
    pub do_not_claim: Vec<String>,
    pub confirm_with: Vec<String>,
    pub effect_direction: EffectDirection,
    pub raw_dna_limitation: Option<String>,
    pub clinical_confirmation_required: Option<bool>,
    pub sources: Option<Vec<MarkerSource>>,
    pub variant_type: Option<String>,
    pub severity_class: String,
    pub expected_plus_alleles: Option<Vec<String>>,
    pub strand: Option<String>,
    pub source_build: Option<String>,
    pub hgvs: Option<String>,
    pub allele_orientation_verified: Option<bool>,
    pub orientation_source: Option<String>,
    pub interpretation_blocked_if_unverified: Option<bool>,
}

/// Direction-aware summary statistics for a report section.
#[derive(Debug, Serialize)]
pub struct SectionSummary {
    pub risk_effect_count: u16,
    pub risk_possible: u16,
    pub protective_effect_count: u16,
    pub protective_possible: u16,
    pub trait_count: u16,
    pub context_dependent_count: u16,
    pub no_data_count: u16,
    pub confirmation_required_count: u16,
    pub total_markers: u16,
    pub show_percent_score: bool,
    pub active_marker_count: u16,
    pub active_risk_marker_count: u16,
    pub active_protective_marker_count: u16,
    pub active_trait_marker_count: u16,
    pub active_context_marker_count: u16,
    pub blocked_unverified_count: u16,
    pub benign_modifier_count: u16,
}

#[derive(Debug, Serialize)]
pub struct EvaluatedSection {
    pub name: String,
    pub markers: Vec<EvaluatedMarker>,
    /// Risk-direction-only signal score (0-100%). Meaningless when
    /// `summary.show_percent_score` is false.
    pub section_signal_score: f64,
    /// Detailed direction-aware tallies.
    pub summary: SectionSummary,
}

#[derive(Debug, Serialize)]
pub struct GeneratedReport {
    pub title: String,
    pub description: String,
    pub sections: Vec<EvaluatedSection>,
    /// Risk-direction-only overall signal score.
    pub overall_signal_score: f64,
}

// ---------------------------------------------------------------------------
// Severity classification
// ---------------------------------------------------------------------------

/// Determine the visual severity class for a single marker.
fn compute_severity_class(
    effect_count: u8,
    direction: &EffectDirection,
    _evidence_tier: &str,
    clinical_confirmation_required: Option<bool>,
    is_missing: bool,
) -> String {
    // No data
    if is_missing {
        return "no_data".to_string();
    }

    // No effect alleles detected → benign presentation regardless of direction
    if effect_count == 0 {
        return "benign".to_string();
    }

    // Clinical confirmation takes precedence over direction styling
    if clinical_confirmation_required == Some(true) {
        return "confirmation_required".to_string();
    }

    // Direction-based classification
    match direction {
        EffectDirection::Protective => "protective".to_string(),
        EffectDirection::Trait => "trait".to_string(),
        EffectDirection::ContextDependent => "context_dependent".to_string(),
        EffectDirection::Unknown => "context_dependent".to_string(),
        EffectDirection::Risk => {
            if effect_count >= 2 {
                "high_risk".to_string()
            } else {
                "moderate_risk".to_string()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Report generation
// ---------------------------------------------------------------------------

/// Evaluates a template against a user's database records.
pub fn generate_report(
    conn: &Connection,
    sample_id: i64,
    template: &ReportTemplate,
) -> Result<GeneratedReport, String> {
    // 1. Collect all rsIDs to query in a single batch
    let mut rsids = Vec::new();
    for sec in &template.sections {
        for m in &sec.markers {
            rsids.push(m.rsid.clone());
        }
    }

    // 2. Query user genotypes from database
    let user_variants = crate::db::query_by_rsids(conn, sample_id, &rsids)
        .map_err(|e| format!("Database query error: {}", e))?;

    let mut genotype_map = HashMap::new();
    for v in user_variants {
        let clean_allele1 = if v.allele1.is_empty() { "-".to_string() } else { v.allele1 };
        let clean_allele2 = if v.allele2.is_empty() { "-".to_string() } else { v.allele2 };
        genotype_map.insert(v.rsid.clone(), format!("{}{}", clean_allele1, clean_allele2));
    }

    // 3. Evaluate each section with direction-aware scoring
    let mut evaluated_sections = Vec::new();
    let mut total_risk_possible: u16 = 0;
    let mut total_risk_effects: u16 = 0;
    let mut evaluated_rsids = std::collections::HashSet::new();

    for sec in &template.sections {
        let mut evaluated_markers = Vec::new();

        // Section-level summary accumulators
        let mut risk_effect_count: u16 = 0;
        let mut risk_possible: u16 = 0;
        let mut protective_effect_count: u16 = 0;
        let mut protective_possible: u16 = 0;
        let mut trait_count: u16 = 0;
        let mut context_dependent_count: u16 = 0;
        let mut no_data_count: u16 = 0;
        let mut confirmation_required_count: u16 = 0;
        let mut all_require_confirmation = true;

        let mut active_marker_count: u16 = 0;
        let mut active_risk_marker_count: u16 = 0;
        let mut active_protective_marker_count: u16 = 0;
        let mut active_trait_marker_count: u16 = 0;
        let mut active_context_marker_count: u16 = 0;
        let mut blocked_unverified_count: u16 = 0;
        let mut benign_modifier_count: u16 = 0;

        for m in &sec.markers {
            let genotype = genotype_map
                .get(&m.rsid)
                .cloned()
                .unwrap_or_else(|| "--".to_string());

            let is_missing = genotype == "--"
                || genotype.contains('-')
                || genotype.contains('0')
                || genotype.contains('?');

            // Count effect alleles
            let mut effect_count: u8 = 0;
            if !is_missing && !m.effect_allele.is_empty() {
                let effect_char = m.effect_allele.chars().next().unwrap_or('-');
                for c in genotype.chars() {
                    if c == effect_char {
                        effect_count += 1;
                    }
                }
            }

            // Track confirmation requirement
            let requires_confirmation = m.clinical_confirmation_required == Some(true);
            if requires_confirmation {
                confirmation_required_count += 1;
            } else {
                all_require_confirmation = false;
            }

            // Direction-aware tallying (only for non-missing genotypes)
            if is_missing {
                no_data_count += 1;
            } else {
                match m.effect_direction {
                    EffectDirection::Risk => {
                        risk_possible += 2;
                        risk_effect_count += effect_count as u16;
                        if evaluated_rsids.insert(m.rsid.clone()) {
                            total_risk_possible += 2;
                            total_risk_effects += effect_count as u16;
                        }
                    }
                    EffectDirection::Protective => {
                        protective_possible += 2;
                        protective_effect_count += effect_count as u16;
                    }
                    EffectDirection::Trait => {
                        if effect_count > 0 {
                            trait_count += 1;
                        }
                    }
                    EffectDirection::ContextDependent | EffectDirection::Unknown => {
                        if effect_count > 0 {
                            context_dependent_count += 1;
                        }
                    }
                }
            }

            // Verify allele orientation
            let mut orientation_warning = false;
            if !is_missing
                && let Some(ref expected) = m.expected_plus_alleles {
                    for c in genotype.chars() {
                        let c_str = c.to_string();
                        if !expected.contains(&c_str) {
                            orientation_warning = true;
                        }
                    }
                }

            // Compute severity class
            let mut severity_class = compute_severity_class(
                effect_count,
                &m.effect_direction,
                &m.evidence_tier,
                m.clinical_confirmation_required,
                is_missing,
            );

            // Apply DPYD / Allele-Orientation Safety Gate
            let mut interpretation = m.interpretation.clone();
            let mut impact = m.impact.clone();
            let is_blocked = m.interpretation_blocked_if_unverified == Some(true)
                && (m.allele_orientation_verified != Some(true) || orientation_warning);

            if is_blocked && !is_missing {
                interpretation = "⚠️ Clinical interpretation blocked: Allele orientation has not been verified for this chip build/strand configuration. Confirm genotype with clinical assay.".to_string();
                impact = "Interpretation Blocked (Unverified Strand)".to_string();
                severity_class = "confirmation_required".to_string();
            } else if !is_blocked && orientation_warning && !is_missing {
                interpretation = format!("⚠️ WARNING: Orientation mismatch detected. {}", interpretation);
            }

            // Tally summary categories
            if !is_missing {
                active_marker_count += 1;
                if is_blocked {
                    blocked_unverified_count += 1;
                } else if effect_count == 0 {
                    benign_modifier_count += 1;
                } else {
                    match m.effect_direction {
                        EffectDirection::Risk => active_risk_marker_count += 1,
                        EffectDirection::Protective => active_protective_marker_count += 1,
                        EffectDirection::Trait => active_trait_marker_count += 1,
                        EffectDirection::ContextDependent | EffectDirection::Unknown => {
                            active_context_marker_count += 1;
                        }
                    }
                }
            }

            evaluated_markers.push(EvaluatedMarker {
                rsid: m.rsid.clone(),
                gene: m.gene.clone(),
                variant_name: m.variant_name.clone(),
                user_genotype: genotype,
                effect_allele: m.effect_allele.clone(),
                effect_count,
                impact,
                evidence_tier: m.evidence_tier.clone(),
                interpretation,
                do_not_claim: m.do_not_claim.clone(),
                confirm_with: m.confirm_with.clone(),
                effect_direction: m.effect_direction.clone(),
                raw_dna_limitation: m.raw_dna_limitation.clone(),
                clinical_confirmation_required: m.clinical_confirmation_required,
                sources: m.sources.clone(),
                variant_type: m.variant_type.clone(),
                severity_class,
                expected_plus_alleles: m.expected_plus_alleles.clone(),
                strand: m.strand.clone(),
                source_build: m.source_build.clone(),
                hgvs: m.hgvs.clone(),
                allele_orientation_verified: m.allele_orientation_verified,
                orientation_source: m.orientation_source.clone(),
                interpretation_blocked_if_unverified: m.interpretation_blocked_if_unverified,
            });
        }

        // Determine if this section should display a percent score
        let show_percent_score = !all_require_confirmation && risk_possible > 0;

        let section_signal_score = if risk_possible > 0 {
            (risk_effect_count as f64 / risk_possible as f64) * 100.0
        } else {
            0.0
        };

        let summary = SectionSummary {
            risk_effect_count,
            risk_possible,
            protective_effect_count,
            protective_possible,
            trait_count,
            context_dependent_count,
            no_data_count,
            confirmation_required_count,
            total_markers: sec.markers.len() as u16,
            show_percent_score,
            active_marker_count,
            active_risk_marker_count,
            active_protective_marker_count,
            active_trait_marker_count,
            active_context_marker_count,
            blocked_unverified_count,
            benign_modifier_count,
        };

        evaluated_sections.push(EvaluatedSection {
            name: sec.name.clone(),
            markers: evaluated_markers,
            section_signal_score,
            summary,
        });
    }

    let overall_signal_score = if total_risk_possible > 0 {
        (total_risk_effects as f64 / total_risk_possible as f64) * 100.0
    } else {
        0.0
    };

    Ok(GeneratedReport {
        title: template.title.clone(),
        description: template.description.clone(),
        sections: evaluated_sections,
        overall_signal_score,
    })
}

// ---------------------------------------------------------------------------
// Markdown renderer
// ---------------------------------------------------------------------------

/// Helper to render the generated report as a readable markdown string.
pub fn render_markdown(report: &GeneratedReport) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", report.title));
    md.push_str(&format!(">{}\n\n", report.description));
    md.push_str(&format!(
        "**Risk-Direction Signal Score**: {:.1}%\n\n",
        report.overall_signal_score
    ));
    md.push_str("> *This score reflects only risk-direction markers. Protective, trait, and context-dependent markers are tallied separately.*\n\n");
    md.push_str("---\n\n");

    for sec in &report.sections {
        md.push_str(&format!("## {}\n", sec.name));

        if sec.summary.show_percent_score {
            md.push_str(&format!(
                "*Risk Signal Score*: {:.1}%\n\n",
                sec.section_signal_score
            ));
        } else {
            md.push_str("*Score suppressed — clinical confirmation required for all markers in this section.*\n\n");
        }

        let s = &sec.summary;
        md.push_str(&format!(
            "Summary: {} markers | {} risk alleles/{} possible | {} protective | {} trait | {} context-dependent | {} no-data | {} confirmation-required\n\n",
            s.total_markers, s.risk_effect_count, s.risk_possible,
            s.protective_effect_count, s.trait_count,
            s.context_dependent_count, s.no_data_count, s.confirmation_required_count
        ));

        md.push_str("| Marker | Gene | Genotype | Effect Allele | Severity | Direction | Tier | Interpretation |\n");
        md.push_str("|---|---|---|---|---|---|---|---|\n");

        for m in &sec.markers {
            let dir_str = match m.effect_direction {
                EffectDirection::Risk => "Risk",
                EffectDirection::Protective => "Protective",
                EffectDirection::ContextDependent => "Context-dependent",
                EffectDirection::Trait => "Trait",
                EffectDirection::Unknown => "Unknown",
            };
            md.push_str(&format!(
                "| **{}** | **{}** | `{}` | `{}` | {} | {} | `{}` | *{}* |\n",
                m.rsid, m.gene, m.user_genotype, m.effect_allele,
                m.severity_class, dir_str, m.evidence_tier, m.impact
            ));
        }
        md.push_str("\n---\n\n");
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE samples (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                genetic_sex TEXT DEFAULT 'Unknown',
                imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE genotypes (
                sample_id INTEGER,
                rsid TEXT NOT NULL,
                chromosome TEXT NOT NULL,
                position_grch37 INTEGER NOT NULL,
                position_grch38 INTEGER,
                allele1 TEXT NOT NULL,
                allele2 TEXT NOT NULL,
                PRIMARY KEY (sample_id, rsid)
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO samples (id, name) VALUES (1, 'Test Sample')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_dpyd_rs55886062_safety_gate() {
        let conn = setup_test_db();
        let sample_id = 1;

        // Insert homozygous A/A genotype (which is Ref/Ref on minus)
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs55886062",
                "1",
                "97000000",
                "97000000",
                "A",
                "A",
            ],
        )
        .unwrap();

        let marker = MarkerDefinition {
            rsid: "rs55886062".to_string(),
            gene: "DPYD".to_string(),
            variant_name: Some("rs55886062 / I560S".to_string()),
            effect_allele: "C".to_string(), // correct effect allele
            impact: "Severe toxicity".to_string(),
            evidence_tier: "A".to_string(),
            interpretation: "Carriers of C allele have severe toxicity risk.".to_string(),
            do_not_claim: vec![],
            confirm_with: vec![],
            effect_direction: EffectDirection::Risk,
            raw_dna_limitation: None,
            clinical_confirmation_required: Some(true),
            sources: None,
            variant_type: Some("snp".to_string()),
            expected_plus_alleles: Some(vec!["A".to_string(), "C".to_string()]),
            strand: Some("minus".to_string()),
            source_build: Some("GRCh38".to_string()),
            hgvs: Some("c.1679T>G".to_string()),
            allele_orientation_verified: Some(true),
            orientation_source: Some("dbSNP".to_string()),
            interpretation_blocked_if_unverified: Some(true),
        };

        let template = ReportTemplate {
            title: "Test".to_string(),
            description: "Test".to_string(),
            sections: vec![SectionDefinition {
                name: "PGx".to_string(),
                markers: vec![marker.clone()],
            }],
        };

        let report = generate_report(&conn, sample_id, &template).unwrap();
        let evaluated = &report.sections[0].markers[0];

        // Genotype AA is benign because effect allele is C (0 count)
        assert_eq!(evaluated.user_genotype, "AA");
        assert_eq!(evaluated.effect_count, 0);
        assert_eq!(evaluated.severity_class, "benign");

        // Now test genotype GG (G is not in expected alleles [A, C])
        conn.execute("DELETE FROM genotypes", []).unwrap();
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs55886062",
                "1",
                "97000000",
                "97000000",
                "G",
                "G",
            ],
        )
        .unwrap();

        let report2 = generate_report(&conn, sample_id, &template).unwrap();
        let evaluated2 = &report2.sections[0].markers[0];

        // Genotype GG triggers orientation warning and safety gate block
        assert_eq!(evaluated2.user_genotype, "GG");
        assert_eq!(evaluated2.severity_class, "confirmation_required");
        assert!(evaluated2.interpretation.contains("blocked"));
    }
}
