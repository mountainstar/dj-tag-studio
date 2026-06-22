import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { DbConnectionTest, RekordboxStatus, SettingsView } from "../types";

type Props = {
  onClose: () => void;
  onSaved: () => void;
  status: RekordboxStatus | null;
};

export function SettingsPanel({ onClose, onSaved, status }: Props) {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [customPath, setCustomPath] = useState("");
  const [testResult, setTestResult] = useState<DbConnectionTest | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const loadSettings = useCallback(async () => {
    const view = await api.getSettings();
    setSettings(view);
    setCustomPath(view.custom_master_db_path ?? "");
  }, []);

  useEffect(() => {
    loadSettings().catch((e) => setError(String(e)));
  }, [loadSettings]);

  const handleTest = async () => {
    setBusy(true);
    setError(null);
    try {
      const path =
        customPath.trim() ||
        settings?.default_master_db_path ||
        settings?.resolved_master_db_path ||
        "";
      const result = await api.testDbConnection(path);
      setTestResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleUseDefault = () => {
    setCustomPath("");
    setTestResult(null);
  };

  const handleSave = async () => {
    setBusy(true);
    setError(null);
    try {
      const saved = await api.saveSettings(customPath.trim() || null);
      setSettings(saved);
      setCustomPath(saved.custom_master_db_path ?? "");
      onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal settings-modal" onClick={(e) => e.stopPropagation()}>
        <h2>Settings — Rekordbox connection</h2>

        <section className="settings-section">
          <h3>Connection status</h3>
          <ul className="settings-status-list">
            <li>
              <span className="settings-label">Rekordbox app</span>
              <span className={status?.running ? "settings-warn" : "settings-ok"}>
                {status?.running ? "Running (read-only)" : "Not running"}
              </span>
            </li>
            <li>
              <span className="settings-label">Library database</span>
              <span className={status?.db_found ? "settings-ok" : "settings-warn"}>
                {status?.db_found ? "Connected" : "Not found — demo mode"}
              </span>
            </li>
            {status?.db_path && (
              <li className="settings-path-row">
                <span className="settings-label">Active path</span>
                <code className="settings-path">{status.db_path}</code>
              </li>
            )}
          </ul>
        </section>

        <section className="settings-section">
          <h3>Database path</h3>
          <p className="hint">
            DJ Tag Studio opens Rekordbox&apos;s encrypted <code>master.db</code> file.
            Leave custom path empty to use the default location for your OS.
          </p>
          {settings?.default_master_db_path && (
            <p className="settings-default">
              Default: <code>{settings.default_master_db_path}</code>
            </p>
          )}
          <label className="settings-field">
            <span>Custom path (optional)</span>
            <input
              type="text"
              value={customPath}
              onChange={(e) => {
                setCustomPath(e.target.value);
                setTestResult(null);
              }}
              placeholder={settings?.default_master_db_path ?? "/path/to/master.db"}
              spellCheck={false}
            />
          </label>
          <div className="settings-actions-row">
            <button type="button" onClick={handleUseDefault} disabled={busy}>
              Use default
            </button>
            <button type="button" onClick={handleTest} disabled={busy}>
              Test connection
            </button>
          </div>
          {testResult && (
            <p className={testResult.ok ? "settings-test-ok" : "settings-test-fail"}>
              {testResult.message}
            </p>
          )}
        </section>

        <section className="settings-section settings-instructions">
          <h3>How to connect</h3>
          <ol>
            <li>Install Rekordbox 6 or 7 and import or analyze your library once.</li>
            <li>
              <strong>Quit Rekordbox completely</strong> before writing tags (including
              background agents on macOS).
            </li>
            <li>
              Confirm <code>master.db</code> exists at the default path above, or paste
              your path and click <strong>Test connection</strong>.
            </li>
            <li>
              Click <strong>Save &amp; reload</strong> — the library loads from that
              database.
            </li>
            <li>
              After tagging, open Rekordbox again. My Tags appear in the Rekordbox UI
              and export to USB for CDJ/XDJ.
            </li>
          </ol>
          <p className="hint">
            Full guide: <code>docs/rekordbox-connection.md</code> in the project folder.
          </p>
        </section>

        {error && <p className="schema-error">{error}</p>}

        <div className="modal-actions">
          <button type="button" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button type="button" className="primary" onClick={handleSave} disabled={busy}>
            Save &amp; reload
          </button>
        </div>
      </div>
    </div>
  );
}
