import { useState } from "react";
import { api } from "../api";
import type { TagGroup } from "../types";

interface Props {
  groups: TagGroup[];
  canWrite: boolean;
  onClose: () => void;
  onSchemaChange: (groups: TagGroup[]) => void;
}

export function TagSchemaEditor({
  groups,
  canWrite,
  onClose,
  onSchemaChange,
}: Props) {
  const [newTags, setNewTags] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleAdd(groupId: string) {
    const name = (newTags[groupId] ?? "").trim();
    if (!name) return;
    setBusy(true);
    setError(null);
    try {
      await api.createCustomTag(groupId, name);
      const lib = await api.loadLibrary();
      onSchemaChange(lib.groups);
      setNewTags((prev) => ({ ...prev, [groupId]: "" }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(tagId: string) {
    if (!confirm("Remove this tag from your Rekordbox My Tags schema?")) return;
    setBusy(true);
    setError(null);
    try {
      const updated = await api.deleteCustomTag(tagId);
      onSchemaChange(updated);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal schema-modal" onClick={(e) => e.stopPropagation()}>
        <h2>My Tag Schema</h2>
        <p className="hint">
          Rekordbox allows 4 category groups with unlimited sub-tags. Add custom tags
          here — they sync to Rekordbox when created (close Rekordbox first).
        </p>

        {error && <p className="schema-error">{error}</p>}

        {groups.map((g) => (
          <section key={g.id} className="schema-group">
            <h3>{g.name}</h3>
            <ul className="schema-tag-list">
              {g.tags.map((t) => (
                <li key={t.id}>
                  <span>{t.name}</span>
                  {canWrite && (
                    <button
                      type="button"
                      className="schema-delete"
                      disabled={busy}
                      onClick={() => handleDelete(t.id)}
                      title="Remove tag"
                    >
                      ×
                    </button>
                  )}
                </li>
              ))}
            </ul>
            {canWrite && (
              <div className="schema-add-row">
                <input
                  type="text"
                  placeholder={`New ${g.name} tag…`}
                  value={newTags[g.id] ?? ""}
                  disabled={busy}
                  onChange={(e) =>
                    setNewTags((prev) => ({ ...prev, [g.id]: e.target.value }))
                  }
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleAdd(g.id);
                  }}
                />
                <button
                  type="button"
                  disabled={busy || !(newTags[g.id] ?? "").trim()}
                  onClick={() => handleAdd(g.id)}
                >
                  Add
                </button>
              </div>
            )}
          </section>
        ))}

        <div className="modal-actions">
          <button type="button" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
