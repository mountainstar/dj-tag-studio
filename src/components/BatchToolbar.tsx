interface Props {
  selectedCount: number;
  pendingCount: number;
  rekordboxRunning: boolean;
  onSuggest: () => void;
  onCommit: () => void;
  onApplyLayout: () => void;
  canWrite: boolean;
  writeDisabledReason?: string;
  suggesting?: boolean;
  busy?: boolean;
}

export function BatchToolbar({
  selectedCount,
  pendingCount,
  rekordboxRunning,
  onSuggest,
  onCommit,
  onApplyLayout,
  canWrite,
  writeDisabledReason,
  suggesting = false,
  busy = false,
}: Props) {
  const showPending = rekordboxRunning;
  const showWrite = pendingCount > 0;
  const writeDisabled = !canWrite || busy;

  return (
    <div className="batch-toolbar">
      <span>{selectedCount} selected</span>
      {showPending && (
        <span className="pending">{pendingCount} pending changes</span>
      )}
      <button type="button" onClick={onSuggest} disabled={suggesting || busy}>
        {suggesting ? "Analyzing…" : "Auto-suggest"}
      </button>
      <button
        type="button"
        onClick={onApplyLayout}
        disabled
        title="Removed — Apply default layout created incompatible tags. Use your native Rekordbox tags only."
      >
        Apply default layout
      </button>
      {showWrite && (
        <>
          <button
            type="button"
            className="primary"
            onClick={onCommit}
            disabled={writeDisabled}
            title={writeDisabled ? writeDisabledReason : undefined}
          >
            {busy ? "Writing…" : "Write to Rekordbox"}
          </button>
          {writeDisabled && writeDisabledReason && (
            <span className="write-hint">{writeDisabledReason}</span>
          )}
        </>
      )}
    </div>
  );
}
