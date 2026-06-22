# Rekordbox My Tags Guide

DJ Tag Studio writes directly to Rekordbox **My Tags** in `master.db`. These tags export to USB and work on CDJ-3000 **MY TAG** filters.

## Workflow

1. **Close Rekordbox** before writing tags.
2. Open **DJ Tag Studio** — it loads your library from `master.db`.
3. Tag tracks using the 4-group panel or **Auto-suggest**.
4. Click **Write to Rekordbox** — a backup of `master.db` is created automatically.
5. Open Rekordbox — tags appear in the My Tag panel.
6. Export to USB as usual — tags are available on club gear.

## Four category groups

Rekordbox allows exactly **4 My Tag groups** with unlimited sub-tags each. Default layout:

| Group | Purpose |
|-------|---------|
| Genre | Style and regional tags (House, Latin, Afro…) |
| Components | Vocals, Bass-Heavy, Piano… |
| Situation | Warm-Up, Peak-Time, Closing… |
| Energy | Start, Build, Peak, Sustain, Release |

Use **Apply default layout** to create this schema in Rekordbox.

## USB sync warning

When plugging a USB drive into a different computer, Rekordbox may ask **"Sync with My Tag"**. Choose **Do Not Sync** to keep your desktop tags.

## Club gear (CDJ-3000)

Track Filter → **MY TAG** tab shows the same 4 groups. AND logic applies across groups; OR logic within a group.

## Database location

- **macOS:** `~/Library/Pioneer/rekordbox/master.db`
- **Windows:** `%APPDATA%\Pioneer\rekordbox\master.db`

## Demo mode

If Rekordbox is not installed, the app runs in demo mode with sample tracks. Commit and schema writes require a real Rekordbox library.
