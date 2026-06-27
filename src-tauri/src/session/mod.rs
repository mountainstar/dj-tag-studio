use std::collections::{HashMap, HashSet};

use crate::types::{PendingChange, TagGroup, TagSuggestion};

#[derive(Default)]
pub struct TagSession {
    pub pending: Vec<PendingChange>,
    pub undo_stack: Vec<PendingChange>,
    pub redo_stack: Vec<PendingChange>,
    dismissed: HashSet<String>,
}

pub fn suggestion_key(s: &TagSuggestion) -> String {
    format!("{}|{}|{}", s.track_id, s.group_name, s.tag_name)
}

impl TagSession {
    pub fn apply_change(&mut self, track_id: String, tag_id: String, enabled: bool) {
        self.pending.retain(|c| {
            !(c.track_id == track_id && !c.tag_id.is_empty() && c.tag_id == tag_id)
        });
        let change = PendingChange {
            track_id: track_id.clone(),
            tag_id: tag_id.clone(),
            enabled,
            group_id: None,
            tag_name: None,
        };
        self.pending.push(change.clone());
        self.undo_stack.push(change);
        self.redo_stack.clear();
    }

    pub fn apply_create_and_enable(
        &mut self,
        track_id: String,
        group_id: String,
        tag_name: String,
    ) {
        let tag_name = tag_name.trim().to_string();
        self.pending.retain(|c| {
            !(c.track_id == track_id
                && c.tag_name.as_deref() == Some(tag_name.as_str())
                && c.group_id.as_deref() == Some(group_id.as_str()))
        });
        let change = PendingChange {
            track_id: track_id.clone(),
            tag_id: String::new(),
            enabled: true,
            group_id: Some(group_id),
            tag_name: Some(tag_name),
        };
        self.pending.push(change.clone());
        self.undo_stack.push(change);
        self.redo_stack.clear();
    }

    pub fn effective_tags(&self, track_id: &str, base: &[String], groups: &[TagGroup]) -> Vec<String> {
        let mut tags: Vec<String> = base.to_vec();
        for change in &self.pending {
            if change.track_id != track_id {
                continue;
            }
            let tag_id = if !change.tag_id.is_empty() {
                change.tag_id.clone()
            } else if let (Some(group_id), Some(tag_name)) = (&change.group_id, &change.tag_name) {
                groups
                    .iter()
                    .find(|g| g.id == *group_id)
                    .and_then(|g| {
                        g.tags
                            .iter()
                            .find(|t| t.name.eq_ignore_ascii_case(tag_name))
                            .map(|t| t.id.clone())
                    })
                    .unwrap_or_default()
            } else {
                continue;
            };
            if tag_id.is_empty() {
                continue;
            }
            if change.enabled {
                if !tags.contains(&tag_id) {
                    tags.push(tag_id);
                }
            } else {
                tags.retain(|t| t != &tag_id);
            }
        }
        tags
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Keep only the final change per track/tag (or track/new-tag) key.
    pub fn collapse_pending(pending: &[PendingChange]) -> Vec<PendingChange> {
        let mut collapsed: HashMap<String, PendingChange> = HashMap::new();
        for change in pending {
            let key = if !change.tag_id.is_empty() {
                format!("{}|id|{}", change.track_id, change.tag_id)
            } else {
                format!(
                    "{}|create|{}|{}",
                    change.track_id,
                    change.group_id.as_deref().unwrap_or(""),
                    change.tag_name.as_deref().unwrap_or(""),
                )
            };
            collapsed.insert(key, change.clone());
        }
        collapsed.into_values().collect()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dismissed.clear();
    }

    pub fn dismiss_suggestion(&mut self, s: &TagSuggestion) {
        self.dismissed.insert(suggestion_key(s));
    }

    pub fn filter_suggestions(&self, suggestions: Vec<TagSuggestion>) -> Vec<TagSuggestion> {
        suggestions
            .into_iter()
            .filter(|s| !self.dismissed.contains(&suggestion_key(s)))
            .collect()
    }
}
