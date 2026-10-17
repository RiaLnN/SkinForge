# Architecture

SkinForge reads and patches osu!lazer's Realm database through a small C# bridge, and does everything else (path detection, mixing, preview, UI) in Rust.

## Data flow

```text
osu!lazer client.realm ──(realm-bridge, C# .NET 8)──> JSON metadata
                                                            │
                                                     (skinforge-core, Rust)
                                                            │
                                            ┌───────────────┴───────────────┐
                                            ▼                               ▼
                                   (skinforge desktop GUI)        (skinforge-cli)
                                   egui + live canvas             headless CLI
```

Why a C# bridge: Realm's Dynamic API, which lets us read and patch lazer's database without compiling against lazer's internal classes, is available from .NET (tested with Realm .NET SDK 20.1.0 on osu!lazer database schema version 52). The Rust side calls the bridge as a subprocess and exchanges JSON.

## Repository structure

```text
SkinForge/
├── crates/
│   ├── core/                   # Rust core library & CLI (paths, process guard, mixer, parser)
│   └── gui/                    # Desktop GUI (egui + eframe, live canvas, audio preview)
├── realm-bridge/               # C# Realm Dynamic API bridge (.NET 8)
├── packaging/                  # Bundled release files & instructions
├── .github/workflows/          # CI and multi-platform release workflows
├── docs/                       # Screenshots, demo GIF, architecture notes
├── Cargo.toml                  # Cargo workspace definition
└── LICENSE                     # MIT License
```

## `skinforge-core` modules

| Module | Responsibility |
| --- | --- |
| `paths.rs` | Detects lazer's data directory (`client.realm`, `files/`) on Windows and Linux, including Flatpak |
| `process_guard.rs` | Detects a running osu! to prevent concurrent Realm writes |
| `storage.rs` | Streaming SHA-256 and deduplicated file placement matching lazer's layout (`files/<h0>/<h0h1>/<hash>`) |
| `mixer.rs` | Manifest-based skin mixing that references existing file hashes instead of copying bytes |
| `bridge.rs` | Wrapper around the C# bridge, with a timestamped `client.realm` backup before any write |
| `skin_ini.rs` | Reads, modifies and merges `skin.ini` sections and combo colours |
| `categorize.rs` | Maps skin files and sprites to canonical slots: Cursor, Gameplay, Audio, Interface |

## Zero-copy mixing

lazer identifies files by SHA-256 hash in `files/`. Mixing a cursor from skin A into skin B only adds a reference to the existing hash in B's file list; no bytes are copied. Files that do not exist yet (for example from an external donor folder) are hashed and placed into `files/` first.

## Write safety

1. Refuse to write while osu! is running.
2. Copy `client.realm` (plus `.lock`, `.note`, `.management` if present) to `client.realm.bak-<timestamp>`.
3. Apply the change as one atomic Realm transaction through the bridge.
