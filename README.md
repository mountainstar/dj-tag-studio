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

## Build installers

```bash
npm run tauri build
```

Outputs:
- **macOS:** `src-tauri/target/release/bundle/dmg/`
- **Windows:** `src-tauri/target/release/bundle/msi/`

## Usage

See [docs/rekordbox-my-tags-guide.md](docs/rekordbox-my-tags-guide.md).

**Important:** Close Rekordbox before clicking **Write to Rekordbox**.

## Phase 2 (planned)

- Serato ID3 tag export
- ONNX-based genre/mood classification
- Stem-based component detection
