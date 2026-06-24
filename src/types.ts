export interface MyTagDef {
  id: string;
  name: string;
  group_id: string;
  seq: number;
}

export interface TagGroup {
  id: string;
  name: string;
  seq: number;
  tags: MyTagDef[];
}

export interface Track {
  id: string;
  title: string;
  artist: string;
  album: string;
  genre: string;
  bpm: number;
  path: string;
  playback_path: string;
  playback_available: boolean;
  playback_note?: string | null;
  rating: number;
  comment: string;
  tag_ids: string[];
}

export interface Playlist {
  id: string;
  name: string;
  path: string;
  attribute: number;
  track_count: number;
}

export interface LibraryState {
  db_path: string;
  demo_mode: boolean;
  rekordbox_running: boolean;
  groups: TagGroup[];
  tracks: Track[];
  playlists: Playlist[];
}

export type SortColumn = "title" | "artist" | "genre" | "bpm" | "tags";
export type SortDirection = "asc" | "desc";

export interface TagSuggestion {
  track_id: string;
  tag_id: string;
  tag_name: string;
  group_name: string;
  confidence: number;
  reason: string;
  pending_create?: boolean;
}

export interface RekordboxStatus {
  running: boolean;
  db_path: string | null;
  db_found: boolean;
  demo_mode: boolean;
  default_db_path?: string | null;
  custom_db_path?: string | null;
  using_custom_path?: boolean;
}

export interface SettingsView {
  custom_master_db_path: string | null;
  default_master_db_path: string | null;
  resolved_master_db_path: string | null;
  settings_file_path: string;
}

export interface DbConnectionTest {
  ok: boolean;
  path: string;
  track_count: number | null;
  message: string;
}

export interface TagPack {
  name: string;
  version: string;
  groups: { name: string; tags: string[] }[];
}
