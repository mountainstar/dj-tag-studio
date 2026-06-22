import { useEffect, useState } from "react";
import { api } from "../api";
import type { RekordboxStatus } from "../types";

type Props = {
  onReload?: () => void;
  reloading?: boolean;
  onOpenSettings?: () => void;
};

export function RekordboxStatusBar({ onReload, reloading = false, onOpenSettings }: Props) {
  const [status, setStatus] = useState<RekordboxStatus | null>(null);

  useEffect(() => {
    api.getRekordboxStatus().then(setStatus).catch(console.error);
    const id = setInterval(() => {
      api.getRekordboxStatus().then(setStatus).catch(console.error);
    }, 5000);
    return () => clearInterval(id);
  }, []);

  if (!status) return null;

  return (
    <div className={`status-bar ${status.running ? "warn" : "ok"}`}>
      <span className="status-dot" />
      {status.demo_mode && (
        <span>
          Demo mode — Rekordbox library not found.{" "}
          {onOpenSettings && (
            <button type="button" className="status-link" onClick={onOpenSettings}>
              Open Settings
            </button>
          )}
        </span>
      )}
      {!status.demo_mode && status.running && (
        <span>Rekordbox is running — read-only until you close it.</span>
      )}
      {!status.demo_mode && !status.running && (
        <span>Rekordbox closed — ready to write My Tags.</span>
      )}
      {onReload && !status.demo_mode && (
        <button
          type="button"
          className="status-reload"
          onClick={onReload}
          disabled={reloading}
          title="Re-read master.db and repair tag rows for Rekordbox"
        >
          {reloading ? "Reloading…" : "Reload library"}
        </button>
      )}
      {status.db_path && !status.demo_mode && (
        <span className="db-path">{status.db_path}</span>
      )}
    </div>
  );
}
