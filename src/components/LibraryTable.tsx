import type { SortColumn, SortDirection, Track } from "../types";

interface Props {
  tracks: Track[];
  selectedId: string | null;
  selectedIds: Set<string>;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
  onSelect: (id: string, multi: boolean) => void;
  onSort: (column: SortColumn) => void;
}

const COLUMNS: { key: SortColumn; label: string }[] = [
  { key: "title", label: "Title" },
  { key: "artist", label: "Artist" },
  { key: "genre", label: "Genre" },
  { key: "bpm", label: "BPM" },
  { key: "tags", label: "Tags" },
];

function sortIndicator(active: boolean, direction: SortDirection) {
  if (!active) return "";
  return direction === "asc" ? " ▲" : " ▼";
}

export function LibraryTable({
  tracks,
  selectedId,
  selectedIds,
  sortColumn,
  sortDirection,
  onSelect,
  onSort,
}: Props) {
  return (
    <div className="library-table">
      <table>
        <thead>
          <tr>
            {COLUMNS.map(({ key, label }) => (
              <th
                key={key}
                className={sortColumn === key ? "sorted" : undefined}
                onClick={() => onSort(key)}
              >
                {label}
                {sortIndicator(sortColumn === key, sortDirection)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {tracks.map((track) => {
            const active =
              track.id === selectedId || selectedIds.has(track.id);
            return (
              <tr
                key={track.id}
                className={active ? "selected" : undefined}
                onClick={(e) => onSelect(track.id, e.metaKey || e.ctrlKey)}
              >
                <td>{track.title}</td>
                <td>{track.artist}</td>
                <td>{track.genre}</td>
                <td>{track.bpm > 0 ? Math.round(track.bpm) : "—"}</td>
                <td>{track.tag_ids.length || "—"}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {tracks.length === 0 && (
        <p className="empty">No tracks match the current filter.</p>
      )}
    </div>
  );
}
