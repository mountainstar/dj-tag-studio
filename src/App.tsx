import { useCallback, useEffect, useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "./api";
import { BatchToolbar } from "./components/BatchToolbar";
import { LibraryTable } from "./components/LibraryTable";
import { MyTagPanel } from "./components/MyTagPanel";
import { RekordboxStatusBar } from "./components/RekordboxStatus";
import { TagSchemaEditor } from "./components/TagSchemaEditor";
import { SettingsPanel } from "./components/SettingsPanel";
import { SuggestionsPanel } from "./components/SuggestionsPanel";
import type {
  LibraryState,
  RekordboxStatus,
  SortColumn,
  SortDirection,
  TagSuggestion,
  Track,
} from "./types";
import "./App.css";

function App() {
  const [library, setLibrary] = useState<LibraryState | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [effectiveTags, setEffectiveTags] = useState<Set<string>>(new Set());
  const [suggestions, setSuggestions] = useState<TagSuggestion[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const [playlistId, setPlaylistId] = useState("");
  const [sortColumn, setSortColumn] = useState<SortColumn>("title");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [pendingCount, setPendingCount] = useState(0);
  const [showSchema, setShowSchema] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [suggesting, setSuggesting] = useState(false);
  const [writing, setWriting] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [rekordboxStatus, setRekordboxStatus] = useState<RekordboxStatus | null>(null);

  const load = useCallback(async () => {
    try {
      setError(null);
      const lib = await api.loadLibrary();
      setLibrary(lib);
      setTracks(lib.tracks);
      if (lib.tracks.length > 0) {
        setSelectedId(lib.tracks[0].id);
      }
      setPendingCount(await api.getPendingCount());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleReload = useCallback(async () => {
    setReloading(true);
    try {
      await load();
    } finally {
      setReloading(false);
    }
  }, [load]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const syncStatus = () => {
      api
        .getRekordboxStatus()
        .then((status) => {
          setRekordboxStatus(status);
          setLibrary((lib) =>
            lib
              ? {
                  ...lib,
                  demo_mode: status.demo_mode,
                  rekordbox_running: status.running,
                }
              : lib,
          );
        })
        .catch(console.error);
    };
    syncStatus();
    const id = setInterval(syncStatus, 5000);
    return () => clearInterval(id);
  }, []);

  const refreshTracks = useCallback(async () => {
    const filtered = await api.filterTracks(query, filter, {
      playlistId: playlistId || undefined,
      sortBy: sortColumn,
      sortDir: sortDirection,
    });
    setTracks(filtered);
  }, [query, filter, playlistId, sortColumn, sortDirection]);

  useEffect(() => {
    refreshTracks();
  }, [refreshTracks, library]);

  const refreshEffectiveTags = useCallback(async (trackId: string | null) => {
    if (!trackId) {
      setEffectiveTags(new Set());
      return;
    }
    const tags = await api.getEffectiveTags(trackId);
    setEffectiveTags(new Set(tags));
  }, []);

  useEffect(() => {
    refreshEffectiveTags(selectedId);
  }, [selectedId, pendingCount, refreshEffectiveTags]);

  const suggestedTagIds = useMemo(() => {
    if (!selectedId) return new Set<string>();
    return new Set(
      suggestions
        .filter((s) => s.track_id === selectedId)
        .map((s) => s.tag_id),
    );
  }, [suggestions, selectedId]);

  const handleSelect = (id: string, multi: boolean) => {
    if (multi) {
      setSelectedIds((prev) => {
        const next = new Set(prev);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      });
    } else {
      setSelectedId(id);
      setSelectedIds(new Set());
    }
  };

  const handleToggle = async (tagId: string, enabled: boolean) => {
    const targets =
      selectedIds.size > 0
        ? Array.from(selectedIds)
        : selectedId
          ? [selectedId]
          : [];
    if (targets.length === 0) return;

    const queuesOnly = Boolean(library?.rekordbox_running && !library.demo_mode);
    setError(null);
    if (!queuesOnly) {
      setWriting(true);
    }
    try {
      await api.applyTagToggles(
        targets.map((trackId) => ({ trackId, tagId, enabled })),
      );
      const pending = await api.getPendingCount();
      setPendingCount(pending);
      if (!queuesOnly) {
        setLibrary(await api.getLibrary());
      }
      if (selectedId) await refreshEffectiveTags(selectedId);
    } catch (e) {
      setError(String(e));
    } finally {
      if (!queuesOnly) {
        setWriting(false);
      }
    }
  };

  const handleSuggest = async () => {
    const ids =
      selectedIds.size > 0
        ? Array.from(selectedIds)
        : selectedId
          ? [selectedId]
          : [];
    setSuggesting(true);
    setError(null);
    try {
      const result = await api.getAutoSuggestions(ids);
      setSuggestions(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setSuggesting(false);
    }
  };

  const removeSuggestions = (removed: TagSuggestion[]) => {
    const keys = new Set(
      removed.map((s) => `${s.track_id}|${s.group_name}|${s.tag_name}`),
    );
    setSuggestions((prev) =>
      prev.filter(
        (x) => !keys.has(`${x.track_id}|${x.group_name}|${x.tag_name}`),
      ),
    );
  };

  const handleRejectOne = async (s: TagSuggestion) => {
    await api.rejectSuggestions([s]);
    removeSuggestions([s]);
  };

  const handleRejectSuggestions = async () => {
    const ids =
      selectedIds.size > 0
        ? Array.from(selectedIds)
        : selectedId
          ? [selectedId]
          : [];
    const relevant = suggestions.filter((s) => ids.includes(s.track_id));
    if (relevant.length === 0) return;
    await api.rejectSuggestions(relevant);
    removeSuggestions(relevant);
  };

  const handleAcceptOne = async (s: TagSuggestion) => {
    const queuesOnly = Boolean(library?.rekordbox_running && !library.demo_mode);
    setError(null);
    if (!queuesOnly) {
      setWriting(true);
    }
    try {
      await api.acceptSuggestions([s]);
      setPendingCount(await api.getPendingCount());
      if (!queuesOnly) {
        setLibrary(await api.getLibrary());
      }
      if (selectedId) await refreshEffectiveTags(selectedId);
      setSuggestions((prev) =>
        prev.filter(
          (x) =>
            !(
              x.track_id === s.track_id &&
              x.tag_name === s.tag_name &&
              x.group_name === s.group_name
            ),
        ),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      if (!queuesOnly) {
        setWriting(false);
      }
    }
  };

  const handleAcceptSuggestions = async () => {
    const ids =
      selectedIds.size > 0
        ? Array.from(selectedIds)
        : selectedId
          ? [selectedId]
          : [];
    const relevant = suggestions.filter((s) => ids.includes(s.track_id));
    const queuesOnly = Boolean(library?.rekordbox_running && !library.demo_mode);
    setError(null);
    if (!queuesOnly) {
      setWriting(true);
    }
    try {
      await api.acceptSuggestions(relevant);
      setPendingCount(await api.getPendingCount());
      if (!queuesOnly) {
        setLibrary(await api.getLibrary());
      }
      if (selectedId) await refreshEffectiveTags(selectedId);
      setSuggestions((prev) =>
        prev.filter(
          (s) =>
            !relevant.some(
              (r) =>
                r.track_id === s.track_id &&
                r.tag_name === s.tag_name &&
                r.group_name === s.group_name,
            ),
        ),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      if (!queuesOnly) {
        setWriting(false);
      }
    }
  };

  const handleCommit = async () => {
    setWriting(true);
    setError(null);
    try {
      await api.commitToRekordbox();
      setLibrary(await api.getLibrary());
      setPendingCount(0);
      if (selectedId) await refreshEffectiveTags(selectedId);
    } catch (e) {
      setError(String(e));
    } finally {
      setWriting(false);
    }
  };

  const handleApplyLayout = async () => {
    try {
      const groups = await api.applyDefaultTagPack();
      setLibrary((lib) => (lib ? { ...lib, groups } : lib));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSort = (column: SortColumn) => {
    if (sortColumn === column) {
      setSortDirection((dir) => (dir === "asc" ? "desc" : "asc"));
    } else {
      setSortColumn(column);
      setSortDirection("asc");
    }
  };

  const selectedTrack = library?.tracks.find((t) => t.id === selectedId);
  const canWrite = library ? !library.demo_mode && !library.rekordbox_running : false;
  const writeDisabledReason = !library
    ? "Loading library…"
    : library.demo_mode
      ? "Rekordbox master.db not found — running in demo mode."
      : library.rekordbox_running
        ? "Quit Rekordbox completely (check Activity Monitor for rekordbox and rekordboxagent)."
        : writing
          ? "Write in progress…"
          : undefined;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!library || !selectedId) return;
      const idx = tracks.findIndex((t) => t.id === selectedId);
      if (e.key === "ArrowDown" && idx < tracks.length - 1) {
        setSelectedId(tracks[idx + 1].id);
      }
      if (e.key === "ArrowUp" && idx > 0) {
        setSelectedId(tracks[idx - 1].id);
      }
      if (e.key === "s" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        handleSuggest();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <div className="app">
      <header>
        <h1>DJ Tag Studio</h1>
        <p>Rekordbox My Tag accelerator</p>
      </header>

      <RekordboxStatusBar
        onReload={handleReload}
        reloading={reloading}
        onOpenSettings={() => setShowSettings(true)}
      />

      {error && <div className="error-banner">{error}</div>}

      <BatchToolbar
        selectedCount={selectedIds.size || (selectedId ? 1 : 0)}
        pendingCount={pendingCount}
        rekordboxRunning={Boolean(library?.rekordbox_running && !library.demo_mode)}
        onSuggest={handleSuggest}
        onCommit={handleCommit}
        onApplyLayout={handleApplyLayout}
        canWrite={canWrite}
        writeDisabledReason={writeDisabledReason}
        suggesting={suggesting}
        busy={writing}
      />

      {writing && (
        <div className="writing-banner">Writing to Rekordbox…</div>
      )}

      <div className="filters">
        <input
          type="search"
          placeholder="Search title, artist, genre…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select value={filter} onChange={(e) => setFilter(e.target.value)}>
          <option value="all">All tracks</option>
          <option value="untagged">Untagged only</option>
        </select>
        <select
          value={playlistId}
          onChange={(e) => setPlaylistId(e.target.value)}
          className="playlist-select"
        >
          <option value="">All playlists</option>
          {library?.playlists.map((pl) => (
            <option key={pl.id} value={pl.id}>
              {pl.path} ({pl.track_count})
            </option>
          ))}
        </select>
        <button type="button" onClick={() => setShowSettings(true)}>
          Settings
        </button>
        <button type="button" onClick={() => setShowSchema(true)}>
          Manage tags
        </button>
        {suggestions.length > 0 && (
          <>
            <button type="button" className="primary" onClick={handleAcceptSuggestions}>
              Accept {suggestions.length} suggestions
            </button>
            <button type="button" className="deny-btn" onClick={handleRejectSuggestions}>
              Deny all
            </button>
          </>
        )}
      </div>

      <main className="layout">
        <LibraryTable
          tracks={tracks}
          selectedId={selectedId}
          selectedIds={selectedIds}
          sortColumn={sortColumn}
          sortDirection={sortDirection}
          onSelect={handleSelect}
          onSort={handleSort}
        />

        {library && (
          <MyTagPanel
            groups={library.groups}
            activeTagIds={effectiveTags}
            suggestedTagIds={suggestedTagIds}
            onToggle={handleToggle}
          />
        )}

        <aside className="preview">
          <h3>Preview</h3>
          {selectedTrack ? (
            <>
              <p className="track-title">{selectedTrack.title}</p>
              <p className="track-artist">{selectedTrack.artist}</p>
              <p className="meta">
                {selectedTrack.genre} ·{" "}
                {selectedTrack.bpm > 0
                  ? `${Math.round(selectedTrack.bpm)} BPM`
                  : "No BPM"}
              </p>
              {selectedTrack.path && (
                <audio
                  key={selectedTrack.path}
                  controls
                  src={convertFileSrc(selectedTrack.path)}
                />
              )}
              <p className="comment">{selectedTrack.comment || "No comments"}</p>
              <SuggestionsPanel
                suggestions={suggestions}
                trackId={selectedId}
                onAcceptOne={handleAcceptOne}
                onRejectOne={handleRejectOne}
              />
            </>
          ) : (
            <p>Select a track</p>
          )}
        </aside>
      </main>

      {showSettings && (
        <SettingsPanel
          status={rekordboxStatus}
          onClose={() => setShowSettings(false)}
          onSaved={load}
        />
      )}

      {showSchema && library && (
        <TagSchemaEditor
          groups={library.groups}
          canWrite={canWrite}
          onClose={() => setShowSchema(false)}
          onSchemaChange={(groups) =>
            setLibrary((lib) => (lib ? { ...lib, groups } : lib))
          }
        />
      )}
    </div>
  );
}

export default App;
