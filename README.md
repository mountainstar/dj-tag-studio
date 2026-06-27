# DJ Tag Studio

A desktop app that accelerates **Rekordbox My Tag** workflows — fast tagging, auto-suggestions, and direct writes to `master.db` so tags sync to CDJ/XDJ hardware via normal USB export.

## Features

- Read/write Rekordbox My Tags (`djmdMyTag`, `djmdSongMyTag`)
- 4-group tagging UI matching Rekordbox / CDJ-3000 layout
- Keyboard navigation (↑/↓ between tracks)
- Batch tagging and auto-suggestions (genre, components, situation, energy)
- Automatic `master.db` backup before every write
- Demo mode when Rekordbox is not installed

## Requirements

- macOS or Windows
- [Rust](https://rustup.rs/) (for Tauri backend)
- Node.js 18+
- Rekordbox 6/7 with an existing library (optional — demo mode available)

## Development

```bash
cd ~/Projects/dj-tag-studio
npm install
source ~/.cargo/env
npm run tauri dev
```

## Download

Pre-built installers are on the **[GitHub Releases](https://github.com/mountainstar/dj-tag-studio/releases)** page.

- **macOS (Apple Silicon):** `DJ Tag Studio_*_aarch64.dmg`
- **macOS (Intel):** `DJ Tag Studio_*_x64.dmg` (when available from CI)
- **Windows:** `*.msi` (when available from CI)

On first launch, macOS may block the app (unsigned build). Open **System Settings → Privacy & Security** and choose **Open Anyway**.

## Build installers

```bash
npm run tauri build
```

Outputs:
- **macOS:** `src-tauri/target/release/bundle/dmg/`
- **Windows:** `src-tauri/target/release/bundle/msi/`

## Usage

- **[Connecting to Rekordbox](docs/rekordbox-connection.md)** — database path, settings, troubleshooting
- **[My Tags workflow](docs/rekordbox-my-tags-guide.md)** — tagging categories, USB export, club gear

Open **Settings** in the app to verify your `master.db` path and test the connection.

**Important:** Close Rekordbox before clicking **Write to Rekordbox**.

## Phase 2 (planned)

- Serato ID3 tag export
- ONNX-based genre/mood classification
- Stem-based component detection
