// ./src-tauri/src/research/gnomad/mod.rs
mod types;
mod schema;
mod config;
mod cache;
mod allele_norm;
mod manifest;
mod vcf_parse;
mod graphql;
mod vcf_remote;
mod vcf_local;
mod lookup;
mod batch;
mod validate;
mod readiness;
pub mod commands;

pub use types::*;
pub use manifest::{get_or_build_manifest, manifest_candidate_contigs, GnomadReleaseManifest};
pub use lookup::{apply_gnomad_to_payload, fetch_gnomad_for_enrichment, get_gnomad_context};
pub use batch::prefetch_gnomad_batch;
pub use readiness::{download_missing_gnomad_indexes, get_gnomad_readiness};
pub use schema::migrate_gnomad_schema;