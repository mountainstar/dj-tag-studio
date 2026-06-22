import type { TagGroup } from "../types";

interface Props {
  groups: TagGroup[];
  activeTagIds: Set<string>;
  suggestedTagIds: Set<string>;
  onToggle: (tagId: string, enabled: boolean) => void;
}

export function MyTagPanel({
  groups,
  activeTagIds,
  suggestedTagIds,
  onToggle,
}: Props) {
  return (
    <div className="tag-panel">
      <p className="tag-panel-hint">
        Only native Rekordbox tags are shown. In Situation, &quot;Peak&quot; is a timing slot — Energy uses tags like Chill or Banger. Do not use Apply default layout; it breaks Rekordbox.
      </p>
      {groups.map((group) => (
        <section key={group.id} className="tag-group">
          <h3>{group.name}</h3>
          <div className="tag-buttons">
            {group.tags.map((tag) => {
              const active = activeTagIds.has(tag.id);
              const suggested = suggestedTagIds.has(tag.id);
              return (
                <button
                  key={tag.id}
                  type="button"
                  className={`tag-btn ${active ? "active" : ""} ${suggested ? "suggested" : ""}`}
                  onClick={() => onToggle(tag.id, !active)}
                  title={suggested ? "Auto-suggested" : undefined}
                >
                  {tag.name}
                </button>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}
