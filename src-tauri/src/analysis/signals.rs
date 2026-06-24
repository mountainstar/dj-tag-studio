use std::path::Path;

use lofty::file::TaggedFileExt;
use lofty::prelude::ItemKey;
use lofty::tag::Accessor;

use crate::types::Track;

/// Aggregated text + numeric signals used for scoring.
#[derive(Debug, Clone, Default)]
pub struct TrackSignals {
    pub corpus: String,
    pub genre_field: String,
    pub bpm: f64,
    pub rating: i64,
    pub path_tokens: String,
    /// Lowercased parent folder names from the file path (e.g. "bollywood wedding").
    pub path_folders: String,
    pub raw_title: String,
    pub raw_artist: String,
    pub raw_album: String,
    pub raw_path: String,
}

impl TrackSignals {
    pub fn from_track(track: &Track) -> Self {
        let mut parts = vec![
            track.title.clone(),
            track.artist.clone(),
            track.album.clone(),
            track.genre.clone(),
            track.comment.clone(),
        ];

        if let Some(file_tags) = read_embedded_tags(track.file_path()) {
            parts.push(file_tags.genre);
            parts.push(file_tags.comment);
            parts.push(file_tags.grouping);
            parts.push(file_tags.title);
            parts.push(file_tags.artist);
        }

        let path_tokens = path_to_tokens(track.file_path());
        let path_folders = path_folder_tokens(track.file_path());
        parts.push(path_tokens.clone());
        parts.push(path_folders.clone());

        let corpus = parts.join(" ").to_lowercase();

        Self {
            corpus,
            genre_field: track.genre.to_lowercase(),
            bpm: track.bpm,
            rating: track.rating,
            path_tokens: path_tokens.to_lowercase(),
            path_folders: path_folders.to_lowercase(),
            raw_title: track.title.clone(),
            raw_artist: track.artist.clone(),
            raw_album: track.album.clone(),
            raw_path: track.file_path().to_string(),
        }
    }

    pub fn is_house_context(&self) -> bool {
        self.genre_field.contains("house")
            || self.corpus.contains(" house")
            || self.corpus.starts_with("house")
            || self.corpus.contains("house music")
    }

    /// Best-effort release year from path, album, or corpus (1980–2010).
    pub fn inferred_year(&self) -> Option<i32> {
        for source in [&self.raw_path, &self.raw_album, &self.path_folders] {
            if let Some(y) = extract_year(source) {
                return Some(y);
            }
        }
        extract_year(&self.corpus)
    }

    pub fn title_has_vocal_credits(&self) -> bool {
        let t = self.raw_title.to_lowercase();
        t.contains("feat.")
            || t.contains("ft.")
            || t.contains(" feat ")
            || t.contains(" ft ")
            || t.contains("featuring")
    }

    /// Match non-Latin script hints used for regional inference.
    pub fn script_match(&self, hint: &str) -> Option<String> {
        let text = format!("{} {}", self.raw_title, self.raw_artist);
        let has_devanagari = text.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c));
        let has_arabic = text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c));
        let has_cjk = text.chars().any(|c| {
            ('\u{4E00}'..='\u{9FFF}').contains(&c)
                || ('\u{3040}'..='\u{30FF}').contains(&c)
                || ('\u{AC00}'..='\u{D7AF}').contains(&c)
        });

        match hint {
            "devanagari" if has_devanagari => Some("Devanagari script in title/artist".into()),
            "arabic" if has_arabic => Some("Arabic script in title/artist".into()),
            "cjk" if has_cjk => Some("CJK script in title/artist".into()),
            _ => None,
        }
    }

    pub fn contains_any(&self, keywords: &[&str]) -> Option<String> {
        for kw in keywords {
            if self.corpus.contains(kw) {
                return Some((*kw).to_string());
            }
        }
        None
    }

    pub fn word_match(&self, word: &str) -> bool {
        let w = word.to_lowercase();
        self.corpus.contains(&w) || self.corpus.contains(&w.replace('-', " "))
    }

    /// True when metadata indicates an instrumental/dub version (not "Original Mix").
    pub fn is_instrumental_version(&self) -> bool {
        let t = self.raw_title.to_lowercase();
        if t.contains("instrumental") || t.contains("inst mix") || t.contains("inst. mix") {
            return true;
        }
        if t.contains("no vocal") || t.contains("no-vocal") {
            return true;
        }
        // Dub mix/version only — not bare "dub" in artist names
        t.contains("dub mix") || t.contains("dub version") || t.contains("dub edit")
    }
}

struct EmbeddedTags {
    title: String,
    artist: String,
    genre: String,
    comment: String,
    grouping: String,
}

pub fn read_embedded_comment(path: &str) -> Option<String> {
    read_embedded_tags(path).map(|t| t.comment)
}

fn read_embedded_tags(path: &str) -> Option<EmbeddedTags> {
    let p = Path::new(path);
    if !p.exists() || !p.is_file() {
        return None;
    }
    let tagged = lofty::read_from_path(p).ok()?;
    let tag = tagged.primary_tag()?.clone();

    Some(EmbeddedTags {
        title: tag.title()?.to_string(),
        artist: tag.artist()?.to_string(),
        genre: tag.genre()?.to_string(),
        comment: tag.comment()?.to_string(),
        grouping: tag
            .get_string(&ItemKey::ContentGroup)
            .map(|s| s.to_string())
            .unwrap_or_default(),
    })
}

fn path_to_tokens(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .replace(['_', '-', '.'], " ")
}

fn path_folder_tokens(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|c| c.as_os_str().to_str())
                .filter(|s| !s.is_empty() && *s != "/" && *s != ".")
                .map(|s| s.replace(['_', '-'], " "))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn extract_year(text: &str) -> Option<i32> {
    for token in text.split(|c: char| !c.is_ascii_digit()) {
        if token.len() == 4 {
            if let Ok(y) = token.parse::<i32>() {
                if (1980..=2010).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    None
}
