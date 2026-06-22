import type { TagSuggestion } from "../types";

interface Props {
  suggestions: TagSuggestion[];
  trackId: string | null;
  onAcceptOne: (s: TagSuggestion) => void;
  onRejectOne: (s: TagSuggestion) => void;
}

export function SuggestionsPanel({
  suggestions,
  trackId,
  onAcceptOne,
  onRejectOne,
}: Props) {
  const visible = trackId
    ? suggestions.filter((s) => s.track_id === trackId)
    : suggestions;

  if (visible.length === 0) return null;

  return (
    <div className="suggestions-panel">
      <h3>Suggestions</h3>
      <ul>
        {visible.map((s) => (
          <li
            key={`${s.track_id}-${s.tag_id || s.tag_name}-${s.group_name}`}
            className={s.pending_create ? "suggestion-new-tag" : undefined}
          >
            <div className="suggestion-main">
              <span className="suggestion-tag">{s.tag_name}</span>
              <span className="suggestion-group">{s.group_name}</span>
              {s.pending_create && (
                <span className="suggestion-new-badge">New tag</span>
              )}
              <span className="suggestion-conf">
                {Math.round(s.confidence * 100)}%
              </span>
            </div>
            <p className="suggestion-reason">{s.reason}</p>
            <div className="suggestion-actions">
              <button type="button" className="primary" onClick={() => onAcceptOne(s)}>
                {s.pending_create ? "Add tag & apply" : "Accept"}
              </button>
              <button type="button" className="deny-btn" onClick={() => onRejectOne(s)}>
                Deny
              </button>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
