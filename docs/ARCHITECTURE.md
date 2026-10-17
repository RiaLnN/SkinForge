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

## Integration modes

SkinForge supports two workflows for deploying mixed skins into osu!lazer:

1. **Standard `.osk` Packaging (Recommended)**:
   - Implemented in `mixer::export_osk` and `mixer::launch_osk_file`.
   - Packages all resolved sprites, sounds, and generated `skin.ini` into a standard ZIP archive located in `<lazer_root>/exports/<skin_name>.osk`.
   - Automatically forwards the `.osk` file to osu!lazer via `xdg-open` / Flatpak file forwarding on Linux, `cmd /c start` on Windows, or `open` on macOS.
   - **Safety**: 100% database-safe. Never touches `client.realm` directly, eliminating any risk of database corruption or file locks even while osu!lazer is active.

2. **Direct `client.realm` Injection (Zero-Copy)**:
   - Implemented in `mixer::build_patch` and `bridge::apply_patch`.
   - Modifies `client.realm` directly by registering the new skin manifest with references to existing SHA-256 hashes in `files/`.
   - Avoids file duplication and applies instantly without an import cycle.
   - **Safety**:
     1. Strict process guard check (`process_guard::ensure_not_running`): refuses to write while osu!lazer is running to avoid database corruption or lock collisions.
     2. Automatic backup of `client.realm` (plus `.lock`, `.note`, and `.management` if present) to `client.realm.bak-<timestamp>`.
     3. Atomic transaction execution via the Realm .NET dynamic bridge.
