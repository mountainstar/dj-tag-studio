use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagGroup {
    pub id: String,
    pub name: String,
    pub seq: i64,
    pub tags: Vec<MyTagDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyTagDef {
    pub id: String,
    pub name: String,
    pub group_id: String,
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub bpm: f64,
    /// Rekordbox `FolderPath` (may be a cloud virtual path).
    pub path: String,
    /// Best local path for preview/analysis when available.
    #[serde(default)]
    pub playback_path: String,
    #[serde(default)]
    pub playback_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_note: Option<String>,
    pub rating: i64,
    pub comment: String,
    pub tag_ids: Vec<String>,
}

impl Track {
    pub fn file_path(&self) -> &str {
        if !self.playback_path.is_empty() {
            &self.playback_path
        } else {
            &self.path
        }
    }
}

impl Default for Track {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            bpm: 0.0,
            path: String::new(),
            playback_path: String::new(),
            playback_available: false,
            playback_note: None,
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub path: String,
    pub attribute: i64,
    pub track_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryState {
    pub db_path: String,
    pub demo_mode: bool,
    pub rekordbox_running: bool,
    pub groups: Vec<TagGroup>,
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    pub track_id: String,
    pub tag_id: String,
    pub enabled: bool,
    /// Set when the tag must be created in Rekordbox during commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub tracks_changed: usize,
    pub tags_added: usize,
    pub tags_removed: usize,
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSuggestion {
    pub track_id: String,
    pub tag_id: String,
    pub tag_name: String,
    pub group_name: String,
    pub confidence: f64,
    pub reason: String,
    /// True when this tag does not exist in Rekordbox yet and must be created first.
    #[serde(default)]
    pub pending_create: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagPack {
    pub name: String,
    pub version: String,
    pub groups: Vec<TagPackGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagPackGroup {
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekordboxStatus {
    pub running: bool,
    pub db_path: Option<String>,
    pub db_found: bool,
    pub demo_mode: bool,
    pub default_db_path: Option<String>,
    pub custom_db_path: Option<String>,
    pub using_custom_path: bool,
}
