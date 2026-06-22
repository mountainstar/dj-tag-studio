# Connecting DJ Tag Studio to Rekordbox

DJ Tag Studio reads and writes **My Tags** directly in Rekordbox’s encrypted library database (`master.db`). There is no separate login — if the app can find and open that file, you are connected.

## Quick checklist

1. Install **Rekordbox 6 or 7** and let it finish analyzing your library at least once.
2. **Quit Rekordbox completely** before writing tags (Activity Monitor → quit `rekordbox` and `rekordboxagent` on macOS).
3. Open **DJ Tag Studio** → **Settings** and confirm the status shows your library (not Demo mode).
4. Click **Reload library** after changing paths or after tagging in Rekordbox.

## Default database location

| Platform | Path |
|----------|------|
| **macOS** | `~/Library/Pioneer/rekordbox/master.db` |
| **Windows** | `%APPDATA%\Pioneer\rekordbox\master.db` |

The status bar at the top of the app shows the path in use when connected.

## Settings

Open **Settings** from the filter bar (next to **Manage tags**).

- **Default location** — where Rekordbox normally stores `master.db` on your machine.
- **Custom path** — use this if your library lives elsewhere (external drive, alternate Rekordbox profile, copied backup for testing).
- **Test connection** — verifies the file exists, decrypts correctly, and reports track count.
- **Save & reload** — stores your preference and reloads the library.

Leave **Custom path** blank to use the default location automatically.

## Connection states

| Status | Meaning |
|--------|---------|
| **Demo mode** | `master.db` not found. Sample tracks only; writes are disabled. |
| **Rekordbox is running** | Library loaded read-only. Close Rekordbox to write tags. |
| **Rekordbox closed — ready to write** | Connected and ready to commit My Tag changes. |

## First-time setup

### macOS

1. Open Rekordbox and confirm your tracks appear.
2. Quit Rekordbox (Cmd+Q, not just close the window).
3. Launch DJ Tag Studio.
4. If you see **Demo mode**, open **Settings**:
   - Check that `~/Library/Pioneer/rekordbox/master.db` exists in Finder (Go → Go to Folder…).
   - Paste the full path into **Custom path** if needed, then **Test connection**.
5. Click **Save & reload**.

### Windows

1. Open Rekordbox and confirm your library loads.
2. Exit Rekordbox from the system tray if it keeps running in the background.
3. Launch DJ Tag Studio.
4. If needed, set **Custom path** to  
   `C:\Users\<You>\AppData\Roaming\Pioneer\rekordbox\master.db`
5. **Test connection** → **Save & reload**.

## Writing tags safely

1. **Close Rekordbox** before toggling tags that write immediately, or before **Write to Rekordbox**.
2. The app creates a timestamped backup of `master.db` before each write.
3. Open Rekordbox again — My Tags should match what you set in DJ Tag Studio.
4. Use **Reload library** if Rekordbox was open elsewhere or tags look stale.

## Troubleshooting

### “Demo mode — library not found”

- Rekordbox may not be installed, or the library has never been created.
- Open Rekordbox once, then quit and reload DJ Tag Studio.
- In **Settings**, verify the path with **Test connection**.

### “Database is locked” / write fails

- Rekordbox (or `rekordboxagent`) is still running. Quit completely and retry.
- On macOS, check Activity Monitor for Pioneer processes.

### “File found but not a valid Rekordbox master.db”

- You pointed at the wrong file. Select `master.db`, not `master.db-wal`, a backup, or `exportLibrary.xml`.
- XML export does **not** contain My Tags — only `master.db` works.

### Tags don’t appear in Rekordbox

- Click **Reload library** in DJ Tag Studio after external changes.
- In Rekordbox, open the track’s **My Tag** panel (not Genre/Comment fields).
- My Tags do not sync via Rekordbox Cloud; they live only in `master.db` on each machine.

### Library on an external drive

1. In Rekordbox, relocate or keep music on the external volume.
2. Find where that Rekordbox install stores `master.db` (usually still under `~/Library/Pioneer/rekordbox/` unless you use a portable copy).
3. If the whole Pioneer folder is on the external drive, set **Custom path** in Settings to that `master.db`.

## Related docs

- [My Tags workflow](rekordbox-my-tags-guide.md) — tagging categories, USB export, club gear.
