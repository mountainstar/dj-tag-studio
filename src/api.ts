import { invoke } from "@tauri-apps/api/core";
import type {
  LibraryState,
  MyTagDef,
  RekordboxStatus,
  TagGroup,
  TagPack,
  TagSuggestion,
  Track,
} from "./types";

export const api = {
  getRekordboxStatus: () => invoke<RekordboxStatus>("get_rekordbox_status"),
  loadLibrary: () => invoke<LibraryState>("load_library"),
  getLibrary: () => invoke<LibraryState>("get_library"),
  getDefaultTagPack: () => invoke<TagPack>("get_default_tag_pack"),
  applyDefaultTagPack: () => invoke<TagGroup[]>("apply_default_tag_pack"),
  applyTagToggles: (
    toggles: { trackId: string; tagId: string; enabled: boolean }[],
  ) => invoke<void>("apply_tag_toggles", { toggles }),
  getEffectiveTags: (trackId: string) =>
    invoke<string[]>("get_effective_tags", { trackId }),
  getPendingCount: () => invoke<number>("get_pending_count"),
  commitToRekordbox: () => invoke<void>("commit_to_rekordbox"),
  filterTracks: (
    query: string,
    filter: string,
    options?: {
      groupId?: string;
      playlistId?: string;
      sortBy?: string;
      sortDir?: string;
    },
  ) =>
    invoke<Track[]>("filter_tracks", {
      query,
      filter,
      groupId: options?.groupId,
      playlistId: options?.playlistId,
      sortBy: options?.sortBy,
      sortDir: options?.sortDir,
    }),
  getAutoSuggestions: (trackIds: string[]) =>
    invoke<TagSuggestion[]>("get_auto_suggestions", { trackIds }),
  acceptSuggestions: (suggestions: TagSuggestion[]) =>
    invoke<number>("accept_suggestions", { suggestions }),
  rejectSuggestions: (suggestions: TagSuggestion[]) =>
    invoke<number>("reject_suggestions", { suggestions }),
  createCustomTag: (groupId: string, name: string) =>
    invoke<MyTagDef>("create_custom_tag", { groupId, name }),
  deleteCustomTag: (tagId: string) => invoke<TagGroup[]>("delete_custom_tag", { tagId }),
  exportTagPack: () => invoke<TagPack>("export_tag_pack"),
};
