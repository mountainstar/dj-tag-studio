use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackPath {
    pub path: String,
    pub available: bool,
    pub note: Option<String>,
}

/// Resolve a local audio path from Rekordbox content path columns.
pub fn resolve_playback_path(
    folder_path: &str,
    org_folder_path: &str,
    rb_local_folder_path: &str,
    share_root: Option<&Path>,
) -> PlaybackPath {
    let folder = folder_path.trim();
    let org = org_folder_path.trim();
    let local = rb_local_folder_path.trim();

    for candidate in [local, org, folder] {
        if candidate.is_empty() {
            continue;
        }
        if let Some(path) = playable_local_file(candidate) {
            return PlaybackPath {
                path,
                available: true,
                note: None,
            };
        }
    }

    if let Some(root) = share_root {
        for candidate in [folder, org, local] {
            if candidate.is_empty() || !is_cloud_virtual_path(candidate) {
                continue;
            }
            let joined = root.join(candidate.trim_start_matches('/'));
            if let Some(path) = playable_local_file(&joined.to_string_lossy()) {
                return PlaybackPath {
                    path,
                    available: true,
                    note: None,
                };
            }
        }
    }

    let display = first_non_empty([local, org, folder]).unwrap_or_default().to_string();
    let note = if is_cloud_virtual_path(folder) && org.is_empty() && local.is_empty() {
        Some("Cloud-only track — use Rekordbox Cloud Library Sync to move it to local storage.".into())
    } else if !display.is_empty() {
        Some("Audio file not found at the resolved local path.".into())
    } else {
        Some("No audio file path in Rekordbox.".into())
    };

    PlaybackPath {
        path: display,
        available: false,
        note,
    }
}

pub fn rekordbox_share_root() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs_home().map(|home| home.join("Library/Pioneer/rekordbox/share"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|appdata| {
            PathBuf::from(appdata)
                .join("Pioneer")
                .join("rekordbox")
                .join("share")
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        dirs_home().map(|home| home.join(".rekordbox/share"))
    }
}

fn playable_local_file(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || is_cloud_virtual_path(trimmed) {
        return None;
    }
    let p = Path::new(trimmed);
    if p.is_absolute() && p.is_file() {
        return Some(trimmed.to_string());
    }
    None
}

pub fn is_cloud_virtual_path(path: &str) -> bool {
    let p = path.trim();
    p.starts_with("/contents_")
        || p.starts_with("/v4/catalog/")
        || p.starts_with("/PIONEER/")
}

fn first_non_empty<'a>(values: [&'a str; 3]) -> Option<&'a str> {
    values.into_iter().find(|v| !v.trim().is_empty())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_org_folder_path_over_cloud_folder_path() {
        let org = "/Users/me/Music/track.mp3";
        let resolved = resolve_playback_path(
            "/contents_4255855956/artist/track.mp3",
            org,
            "",
            None,
        );
        assert_eq!(resolved.path, org);
        assert!(!resolved.available);
    }

    #[test]
    fn detects_cloud_only_without_local_destination() {
        let resolved = resolve_playback_path("/contents_123/track.mp3", "", "", None);
        assert!(!resolved.available);
        assert!(resolved.note.unwrap().contains("Cloud-only"));
    }

    #[test]
    fn uses_existing_local_folder_path() {
        let tmp = std::env::temp_dir();
        let file = tmp.join("dj-tag-studio-playback-test.mp3");
        std::fs::write(&file, b"fake").unwrap();
        let path = file.display().to_string();

        let resolved = resolve_playback_path(
            "/contents_999/fake.mp3",
            "",
            &path,
            None,
        );
        assert!(resolved.available);
        assert_eq!(resolved.path, path);

        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn alpine_air_from_rekordbox_library() {
        let path = format!(
            "{}/Library/Pioneer/rekordbox/master.db",
            std::env::var("HOME").unwrap()
        );
        if !Path::new(&path).exists() {
            return;
        }

        let db = super::super::db::open_database(Path::new(&path), super::super::db::DatabaseMode::ReadOnly)
            .expect("open master.db");
        let tracks = super::super::content::load_tracks(&db).expect("load tracks");
        let alpine = tracks
            .iter()
            .find(|t| t.title.contains("Alpine Air"))
            .expect("Alpine Air in library");

        assert!(alpine.playback_available, "{:?}", alpine.playback_note);
        assert!(alpine.playback_path.contains("Tydian-Heliconia"));
        assert!(alpine.path.starts_with("/contents_"));
    }
}
