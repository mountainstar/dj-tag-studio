mod analysis;
mod export;
mod rekordbox;
mod session;
mod types;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use rekordbox::backup::backup_master_db;
use rekordbox::content::{filter_missing_group, filter_untagged, load_tag_ids_for_tracks, load_tracks, search_tracks};
use rekordbox::db::{detect_master_db_path, mark_my_tag_merge_needed, open_database, repair_song_my_tag_rows, sync_agent_registry, DatabaseMode, DbError};
use rekordbox::demo::demo_library;
use rekordbox::my_tags::{
    add_custom_subtag, apply_tag_pack, commit_changes, delete_custom_subtag, load_tag_groups,
    repair_duplicate_mytag_defs,
};
use rekordbox::playlists::{filter_by_playlist, load_playlists, sort_tracks};
use rekordbox::process::{is_rekordbox_running, rekordbox_write_block_reason};
use session::TagSession;
use tauri::State;
use types::{
    CommitSummary, LibraryState, MyTagDef, PendingChange, RekordboxStatus, TagGroup, TagPack,
    TagSuggestion, Track,
};

struct AppState {
    library: Mutex<Option<LibraryState>>,
    session: Mutex<TagSession>,
    demo_mode: Mutex<bool>,
    playlist_tracks: Mutex<HashMap<String, HashSet<String>>>,
    write_lock: Mutex<()>,
}

#[tauri::command]
fn get_rekordbox_status() -> RekordboxStatus {
    let running = is_rekordbox_running();
    let db_path = detect_master_db_path();
    RekordboxStatus {
        running,
        db_path: db_path.as_ref().map(|p| p.display().to_string()),
        db_found: db_path.is_some(),
        demo_mode: db_path.is_none(),
    }
}

#[tauri::command]
fn get_library(state: State<'_, AppState>) -> Result<LibraryState, String> {
    state
        .library
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Library not loaded".into())
}

#[tauri::command]
fn load_library(state: State<'_, AppState>) -> Result<LibraryState, String> {
    let running = is_rekordbox_running();
    let db_path = detect_master_db_path();

    let (db_path_str, demo_mode, groups, tracks, playlists, playlist_tracks) =
        if let Some(path) = db_path {
            let mode = if running {
                DatabaseMode::ReadOnly
            } else {
                DatabaseMode::ReadWrite
            };
            let db = open_database(&path, mode).map_err(|e| e.to_string())?;
            let mut groups = load_tag_groups(&db).map_err(|e| e.to_string())?;
            let mut tracks = load_tracks(&db).map_err(|e| e.to_string())?;
            let (playlists, playlist_tracks) =
                load_playlists(&db).map_err(|e| e.to_string())?;
            if !running {
                let _ = sync_agent_registry(&db.conn);
                let _ = repair_duplicate_mytag_defs(&db);
                let _ = repair_song_my_tag_rows(&db.conn);
                let _ = mark_my_tag_merge_needed(&db.conn);
                groups = load_tag_groups(&db).map_err(|e| e.to_string())?;
                tracks = load_tracks(&db).map_err(|e| e.to_string())?;
            }
            (
                path.display().to_string(),
                false,
                groups,
                tracks,
                playlists,
                playlist_tracks,
            )
        } else {
            let (groups, tracks, playlists, playlist_tracks) = demo_library();
            ("demo://library".into(), true, groups, tracks, playlists, playlist_tracks)
        };

    let library = LibraryState {
        db_path: db_path_str,
        demo_mode,
        rekordbox_running: running,
        groups,
        tracks,
        playlists,
    };

    *state.library.lock().unwrap() = Some(library.clone());
    *state.playlist_tracks.lock().unwrap() = playlist_tracks;
    state.session.lock().unwrap().clear();
    Ok(library)
}

#[tauri::command]
fn get_default_tag_pack() -> Result<TagPack, String> {
    serde_json::from_str(include_str!("../../tag-packs/rekordbox-default.json"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_default_tag_pack(state: State<'_, AppState>) -> Result<Vec<TagGroup>, String> {
    if let Some(reason) = rekordbox_write_block_reason() {
        return Err(reason);
    }
    let path = detect_master_db_path().ok_or_else(|| {
        "Rekordbox database not found. Demo mode cannot write schema.".to_string()
    })?;
    let pack: TagPack = get_default_tag_pack()?;
    let mut db = open_database(&path, DatabaseMode::ReadWrite).map_err(|e| e.to_string())?;
    backup_master_db(&path).map_err(|e| e.to_string())?;
    let groups = apply_tag_pack(&mut db, &pack).map_err(|e| e.to_string())?;

    if let Some(lib) = state.library.lock().unwrap().as_mut() {
        lib.groups = groups.clone();
    }
    Ok(groups)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagToggle {
    track_id: String,
    tag_id: String,
    enabled: bool,
}

#[tauri::command]
fn apply_tag_toggles(toggles: Vec<TagToggle>, state: State<'_, AppState>) -> Result<(), String> {
    if toggles.is_empty() {
        return Ok(());
    }

    let demo_mode = state
        .library
        .lock()
        .unwrap()
        .as_ref()
        .map(|l| l.demo_mode)
        .unwrap_or(true);
    let rekordbox_running = rekordbox_write_block_reason().is_some();

    if demo_mode || rekordbox_running {
        let mut session = state.session.lock().unwrap();
        for toggle in toggles {
            session.apply_change(toggle.track_id, toggle.tag_id, toggle.enabled);
        }
        return Ok(());
    }

    let changes: Vec<PendingChange> = toggles
        .into_iter()
        .map(|toggle| PendingChange {
            track_id: toggle.track_id,
            tag_id: toggle.tag_id,
            enabled: toggle.enabled,
            group_id: None,
            tag_name: None,
        })
        .collect();
    write_changes_to_db(&state, &changes, false)?;
    Ok(())
}

#[tauri::command]
fn get_effective_tags(track_id: String, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let lib = state.library.lock().unwrap();
    let lib = lib.as_ref().ok_or("Library not loaded")?;
    let track = lib
        .tracks
        .iter()
        .find(|t| t.id == track_id)
        .ok_or("Track not found")?;
    Ok(state
        .session
        .lock()
        .unwrap()
        .effective_tags(&track_id, &track.tag_ids, &lib.groups))
}

fn perform_commit(state: &AppState) -> Result<CommitSummary, String> {
    if let Some(reason) = rekordbox_write_block_reason() {
        return Err(reason);
    }

    let pending = {
        let session = state.session.lock().unwrap();
        if session.pending.is_empty() {
            return Err("No pending changes to commit".into());
        }
        TagSession::collapse_pending(&session.pending)
    };

    let lib = state.library.lock().unwrap();
    let lib = lib.as_ref().ok_or("Library not loaded")?;
    if lib.demo_mode {
        return Err("Demo mode cannot commit to Rekordbox".into());
    }
    drop(lib);

    write_changes_to_db(state, &pending, true)
}

fn write_changes_to_db(
    state: &AppState,
    changes: &[PendingChange],
    clear_pending: bool,
) -> Result<CommitSummary, String> {
    if changes.is_empty() {
        return Err("No changes to write".into());
    }

    if let Some(reason) = rekordbox_write_block_reason() {
        return Err(reason);
    }

    let _write_guard = state
        .write_lock
        .try_lock()
        .map_err(|_| "Another write is already in progress. Please wait.".to_string())?;

    let groups = {
        let lib = state.library.lock().unwrap();
        let lib = lib.as_ref().ok_or("Library not loaded")?;
        if lib.demo_mode {
            return Err("Demo mode cannot commit to Rekordbox".into());
        }
        lib.groups.clone()
    };

    let path = detect_master_db_path().ok_or_else(|| {
        "Rekordbox database not found — demo mode cannot commit".to_string()
    })?;

    let backup_path = backup_master_db(&path).map_err(|e| e.to_string())?;
    let mut db = open_database(&path, DatabaseMode::ReadWrite).map_err(|e| e.to_string())?;

    let (tracks_changed, tags_added, tags_removed) =
        commit_changes(&mut db, changes, &groups).map_err(|e| format_db_error(e))?;

    let changed_track_ids: Vec<String> = changes
        .iter()
        .map(|c| c.track_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let groups = load_tag_groups(&db).map_err(|e| e.to_string())?;
    let tag_patch =
        load_tag_ids_for_tracks(&db, &changed_track_ids).map_err(|e| e.to_string())?;

    if let Some(lib) = state.library.lock().unwrap().as_mut() {
        lib.groups = groups;
        lib.rekordbox_running = false;
        for track in &mut lib.tracks {
            if let Some(tag_ids) = tag_patch.get(&track.id) {
                track.tag_ids = tag_ids.clone();
            }
        }
    }

    if clear_pending {
        state.session.lock().unwrap().clear();
    }

    Ok(CommitSummary {
        tracks_changed,
        tags_added,
        tags_removed,
        backup_path: backup_path.display().to_string(),
    })
}

fn format_db_error(err: DbError) -> String {
    let msg = err.to_string();
    if msg.contains("database is locked") || msg.contains("SQLITE_BUSY") {
        format!(
            "{msg}. Quit Rekordbox completely (including any Pioneer background processes) and try again."
        )
    } else {
        msg
    }
}

#[tauri::command]
fn get_pending_count(state: State<'_, AppState>) -> usize {
    state.session.lock().unwrap().pending_count()
}

#[tauri::command]
fn commit_to_rekordbox(state: State<'_, AppState>) -> Result<CommitSummary, String> {
    perform_commit(&state)
}

#[tauri::command]
fn filter_tracks(
    query: String,
    filter: String,
    group_id: Option<String>,
    playlist_id: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Track>, String> {
    let lib = state.library.lock().unwrap();
    let lib = lib.as_ref().ok_or("Library not loaded")?;
    let mut tracks = lib.tracks.clone();

    if !query.trim().is_empty() {
        tracks = search_tracks(&tracks, &query);
    }

    tracks = match filter.as_str() {
        "untagged" => filter_untagged(&tracks),
        "missing_group" => {
            if let Some(gid) = group_id {
                if let Some(group) = lib.groups.iter().find(|g| g.id == gid) {
                    let ids: Vec<String> = group.tags.iter().map(|t| t.id.clone()).collect();
                    filter_missing_group(&tracks, &ids)
                } else {
                    tracks
                }
            } else {
                tracks
            }
        }
        _ => tracks,
    };

    if let Some(pid) = playlist_id.filter(|id| !id.is_empty()) {
        let membership = state.playlist_tracks.lock().unwrap();
        tracks = filter_by_playlist(&tracks, &membership, &pid);
    }

    let sort_by = sort_by.as_deref().unwrap_or("title");
    let sort_dir = sort_dir.as_deref().unwrap_or("asc");
    sort_tracks(&mut tracks, sort_by, sort_dir);

    Ok(tracks)
}

#[tauri::command]
fn get_auto_suggestions(
    track_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TagSuggestion>, String> {
    let (groups, selected) = {
        let lib = state.library.lock().unwrap();
        let lib = lib.as_ref().ok_or("Library not loaded")?;
        let groups = lib.groups.clone();
        let selected: Vec<Track> = lib
            .tracks
            .iter()
            .filter(|t| track_ids.is_empty() || track_ids.contains(&t.id))
            .cloned()
            .collect();
        (groups, selected)
    };

    let raw = analysis::suggest_for_library(&selected, &groups);
    let session = state.session.lock().unwrap();
    Ok(session.filter_suggestions(raw))
}

#[tauri::command]
fn create_custom_tag(
    group_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<MyTagDef, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Tag name cannot be empty".into());
    }

    let mut lib_guard = state.library.lock().unwrap();
    let lib = lib_guard.as_mut().ok_or("Library not loaded")?;

    ensure_custom_tag(lib, &group_id, &name)
}

#[tauri::command]
fn delete_custom_tag(tag_id: String, state: State<'_, AppState>) -> Result<Vec<TagGroup>, String> {
    if let Some(reason) = rekordbox_write_block_reason() {
        return Err(reason);
    }

    let mut lib_guard = state.library.lock().unwrap();
    let lib = lib_guard.as_mut().ok_or("Library not loaded")?;

    if lib.demo_mode {
        for group in &mut lib.groups {
            group.tags.retain(|t| t.id != tag_id);
        }
        return Ok(lib.groups.clone());
    }

    let path = detect_master_db_path()
        .ok_or("Rekordbox database not found".to_string())?;
    backup_master_db(&path).map_err(|e| e.to_string())?;
    let db = open_database(&path, DatabaseMode::ReadWrite).map_err(|e| e.to_string())?;
    delete_custom_subtag(&db, &tag_id).map_err(|e| e.to_string())?;
    lib.groups = load_tag_groups(&db).map_err(|e| e.to_string())?;
    Ok(lib.groups.clone())
}

fn ensure_custom_tag(lib: &mut LibraryState, group_id: &str, name: &str) -> Result<MyTagDef, String> {
    if let Some(existing) = lib.groups.iter().find_map(|g| {
        if g.id == group_id {
            g.tags
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(name))
                .cloned()
        } else {
            None
        }
    }) {
        return Ok(existing);
    }

    if lib.demo_mode {
        let def = MyTagDef {
            id: uuid::Uuid::new_v4().to_string().replace('-', ""),
            name: name.to_string(),
            group_id: group_id.to_string(),
            seq: lib
                .groups
                .iter()
                .find(|g| g.id == group_id)
                .map(|g| g.tags.len() as i64 + 1)
                .unwrap_or(1),
        };
        if let Some(group) = lib.groups.iter_mut().find(|g| g.id == group_id) {
            group.tags.push(def.clone());
        }
        return Ok(def);
    }

    if let Some(reason) = rekordbox_write_block_reason() {
        return Err(reason);
    }

    let path = detect_master_db_path().ok_or("Rekordbox database not found".to_string())?;
    backup_master_db(&path).map_err(|e| e.to_string())?;
    let db = open_database(&path, DatabaseMode::ReadWrite).map_err(|e| e.to_string())?;
    let def = add_custom_subtag(&db, group_id, name).map_err(|e| format_db_error(e))?;
    lib.groups = load_tag_groups(&db).map_err(|e| e.to_string())?;
    Ok(def)
}

#[tauri::command]
fn accept_suggestions(
    suggestions: Vec<TagSuggestion>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let (groups_snapshot, tracks_snapshot) = {
        let lib = state.library.lock().unwrap();
        let lib = lib.as_ref().ok_or("Library not loaded")?;
        (lib.groups.clone(), lib.tracks.clone())
    };

    let demo_mode = state
        .library
        .lock()
        .unwrap()
        .as_ref()
        .map(|l| l.demo_mode)
        .unwrap_or(true);
    let rekordbox_running = rekordbox_write_block_reason().is_some();

    let mut changes = Vec::new();
    for s in suggestions {
        if s.pending_create || s.tag_id.is_empty() {
            let group_id = groups_snapshot
                .iter()
                .find(|g| g.name == s.group_name)
                .map(|g| g.id.clone())
                .ok_or_else(|| format!("Unknown group: {}", s.group_name))?;

            let existing_id = groups_snapshot.iter().find_map(|g| {
                if g.id == group_id {
                    g.tags
                        .iter()
                        .find(|t| t.name.eq_ignore_ascii_case(&s.tag_name))
                        .map(|t| t.id.clone())
                } else {
                    None
                }
            });

            if let Some(tag_id) = existing_id {
                let already = tracks_snapshot
                    .iter()
                    .find(|t| t.id == s.track_id)
                    .map(|t| t.tag_ids.contains(&tag_id))
                    .unwrap_or(false);
                if already {
                    continue;
                }
                changes.push(PendingChange {
                    track_id: s.track_id.clone(),
                    tag_id,
                    enabled: true,
                    group_id: None,
                    tag_name: None,
                });
            } else {
                changes.push(PendingChange {
                    track_id: s.track_id.clone(),
                    tag_id: String::new(),
                    enabled: true,
                    group_id: Some(group_id),
                    tag_name: Some(s.tag_name.clone()),
                });
            }
            continue;
        }

        let already = tracks_snapshot
            .iter()
            .find(|t| t.id == s.track_id)
            .map(|t| t.tag_ids.contains(&s.tag_id))
            .unwrap_or(false);
        if already {
            continue;
        }

        changes.push(PendingChange {
            track_id: s.track_id.clone(),
            tag_id: s.tag_id.clone(),
            enabled: true,
            group_id: None,
            tag_name: None,
        });
    }

    if changes.is_empty() {
        return Ok(0);
    }

    let queued = changes.len();

    if demo_mode || rekordbox_running {
        let mut session = state.session.lock().unwrap();
        for change in changes {
            if change.tag_id.is_empty() {
                if let (Some(group_id), Some(tag_name)) = (&change.group_id, &change.tag_name) {
                    session.apply_create_and_enable(
                        change.track_id,
                        group_id.clone(),
                        tag_name.clone(),
                    );
                }
            } else {
                session.apply_change(change.track_id, change.tag_id, change.enabled);
            }
        }
        return Ok(queued);
    }

    write_changes_to_db(&state, &changes, false)?;
    Ok(queued)
}

#[tauri::command]
fn reject_suggestions(
    suggestions: Vec<TagSuggestion>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let mut session = state.session.lock().unwrap();
    for s in &suggestions {
        session.dismiss_suggestion(s);
    }
    Ok(suggestions.len())
}

#[tauri::command]
fn export_tag_pack(state: State<'_, AppState>) -> Result<TagPack, String> {
    let lib = state.library.lock().unwrap();
    let lib = lib.as_ref().ok_or("Library not loaded")?;
    Ok(TagPack {
        name: "Exported Tag Pack".into(),
        version: "1.0.0".into(),
        groups: lib
            .groups
            .iter()
            .map(|g| types::TagPackGroup {
                name: g.name.clone(),
                tags: g.tags.iter().map(|t| t.name.clone()).collect(),
            })
            .collect(),
    })
}

#[tauri::command]
fn convert_file_path(path: String) -> Result<String, String> {
    Ok(format!("file://{}", urlencoding_path(&path)))
}

fn urlencoding_path(path: &str) -> String {
    path.replace(' ', "%20")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            library: Mutex::new(None),
            session: Mutex::new(TagSession::default()),
            demo_mode: Mutex::new(false),
            playlist_tracks: Mutex::new(HashMap::new()),
            write_lock: Mutex::new(()),
        })
        .invoke_handler(tauri::generate_handler![
            get_rekordbox_status,
            load_library,
            get_library,
            get_default_tag_pack,
            apply_default_tag_pack,
            apply_tag_toggles,
            get_effective_tags,
            get_pending_count,
            commit_to_rekordbox,
            filter_tracks,
            get_auto_suggestions,
            accept_suggestions,
            reject_suggestions,
            create_custom_tag,
            delete_custom_tag,
            export_tag_pack,
            convert_file_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
