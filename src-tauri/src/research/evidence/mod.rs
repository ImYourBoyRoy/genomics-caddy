// ./src-tauri/src/research/evidence/mod.rs
//! Normalized association evidence workbench — SQLite truth + Qdrant discovery.

pub mod adapters;
pub mod atlas;
pub mod backfill;
pub mod cache;
pub mod card;
pub mod catalog;
pub mod commands;
pub mod corpus;
pub mod dashboard;
pub mod named_vectors;
pub mod ncbi_context;
pub mod normalize;
pub mod ontology;
pub mod packet;
pub mod pgs_match;
pub mod schema;
pub mod scoring;
pub mod search;
pub mod source_records;
pub mod store;
pub mod types;

pub use schema::migrate_evidence_schema;
pub use types::*;
