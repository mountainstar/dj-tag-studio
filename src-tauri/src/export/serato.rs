//! Phase 2: Serato ID3 tag export.
#![allow(dead_code)]
//!
//! Serato stores metadata in standard ID3/Vorbis/MP4 fields plus proprietary
//! GEOB frames. This module will map Rekordbox My Tags to Serato-compatible
//! fields without corrupting cue points or beat grids.
//!
//! Reference: https://github.com/bvandrc/serato-tools

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeratoExportMapping {
    pub genre_field: String,
    pub comment_field: String,
    pub grouping_field: String,
}

impl Default for SeratoExportMapping {
    fn default() -> Self {
        Self {
            genre_field: "TCON".into(),
            comment_field: "COMM".into(),
            grouping_field: "TIT1".into(),
        }
    }
}

/// Placeholder for Phase 2 Serato export pipeline.
pub fn export_tags_to_serato(_paths: &[String], _mapping: &SeratoExportMapping) -> Result<usize, String> {
    Err("Serato export is planned for Phase 2. Use Rekordbox My Tags export for now.".into())
}
