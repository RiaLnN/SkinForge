# Architecture

SkinForge discovers skins from both **osu!lazer** (via a lightweight C# Realm Dynamic API bridge) and **osu!stable** (by scanning filesystem `Skins/` directories), and performs all categorization, cross-version mixing, interactive preview rendering, audio synthesis, and `.osk` packaging in Rust.

## Data flow

```text
osu!lazer client.realm ──(realm-bridge, C# .NET 8)──> Lazer Skins (SHA-256 storage) ┐
                                                                                    ├──> (skinforge-core, Rust)
osu!stable Skins/      ──(filesystem scanner)───────> Stable Skins (folder paths)   ┘             │
                                                                                  ┌───────────────┴───────────────┐
                                                                                  ▼                               ▼
                                                                         (skinforge desktop GUI)        (skinforge-cli)
                                                                         egui + live canvas + audio     headless CLI
```

Why a C# bridge for `osu!lazer`: Realm's Dynamic API lets SkinForge read `osu!lazer`'s `client.realm` database without compiling against lazer's internal C# classes (tested with Realm .NET SDK 20.1.0 on osu!lazer database schema v52). Meanwhile, `osu!stable` skins are scanned directly in Rust from standard `Skins/` directories on Windows and Linux (including Wine/Lutris prefixes).

## Repository structure

```text
SkinForge/
├── crates/
│   ├── core/                   # Rust core library & CLI (lazer/stable discovery, categorization, mixer, parser)
│   └── gui/                    # Desktop GUI (egui + eframe, live playfield canvas, element inspector, audio preview)
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
| `paths.rs` | Detects `osu!lazer` (`client.realm`, `files/`) and `osu!stable` (`Skins/`) directories across Windows and Linux, and scans `stable` skin folders into `Skin` structs |
| `bridge.rs` | Defines `Skin`, `SkinSource` (`Lazer` / `Stable`), `Skin::resolve_file_path`, and the subprocess wrapper around `realm-bridge` |
| `categorize.rs` | Maps skin files and sprites to 5 top-level categories (`Cursor`, `Gameplay`, `Hud`, `Interface`, `Audio`), 15 collapsible `ComponentSubgroup`s, and 42 `ComponentSlot` variants |
| `mixer.rs` | Cross-version skin mixing (`export_osk`), `@2x` retina sprite pruning, `combo-*` / `default-*` digit alias normalization, `.osk` ZIP archive generation, and `launch_exported_osk` (`LaunchTarget::Lazer`, `LaunchTarget::Stable` with automatic Wine prefix & `Z:\` path conversion on Linux, or `LaunchTarget::None`) |
| `skin_ini.rs` | Parses, modifies, and serializes `skin.ini` sections (`[General]`, `[Colours]`, `[Fonts]`) with BOM and comment handling |
| `storage.rs` | Streaming SHA-256 and deduplicated file placement matching lazer's layout (`files/<h0>/<h0h1>/<hash>`) |
| `process_guard.rs` | Detects running `osu!` processes for CLI/database utilities |

## `skinforge-gui` modules

| Module | Responsibility |
| --- | --- |
| `app.rs` | Top-level `eframe::App` state loop, `osu!`-inspired dark-plum & pink theme (`apply_osu_pink_theme`), `.osk` save file dialog dispatch, and top status bar |
| `state.rs` | `AppState`, version filter toggles (`show_lazer_skins`, `show_stable_skins`), `launch_target`, UTF-8 safe `truncate_text`, subgroup expand/collapse state, and `HitJudgementKind` timing logic |
| `textures.rs` | Dual-source `TextureCache` resolving sprites from both `osu!lazer` hashed storage (`files_dir`) and `osu!stable` skin directories (`root_dir`) |
| `audio.rs` | Stateful `AudioPlayer` retaining an active `rodio::Sink` for instant Play/Stop toggling, volume control, and in-memory PCM WAV synthesis for default `osu!` fallback sounds |
| `views/sidebar.rs` | Left-aligned skin list and dropdown helpers with distinct `LAZER` / `STABLE` pill badges, high-contrast version filter buttons, custom folder pickers, and search bar |
| `views/categories.rs` | 6-tab component editor with collapsible sub-groups, unified `LAZER` / `STABLE` donor badges, subgroup batch selectors, mini-thumbnails, and deterministic 4-column slot rows |
| `views/preview.rs` | Interactive 60 fps playfield canvas with natural `300`/`100`/`50`/`Miss` hit judgements on click, Selected Element Inspector, post-export launch target selector, and `.osk` export trigger |
| `views/colors.rs` | Combo colour palette editor and advanced `[Colours]` overrides |

## Cross-version `.osk` export & launch

`mixer::export_osk` resolves every file in the mixed skin via `Skin::resolve_file_path(files_dir, skin_file)`:
- For `SkinSource::Lazer`, bytes are read from `<lazer_root>/files/<h0>/<h0h1>/<sha256>`.
- For `SkinSource::Stable`, bytes are read from `<skin_root_dir>/<filename>`.
- For `SlotSource::CustomFile(path)`, bytes are read directly from disk.

All resolved assets and the generated `skin.ini` are packaged into a standard `.osk` archive at the user's chosen destination. Depending on the selected `LaunchTarget`, `mixer::launch_exported_osk` can then open the `.osk` in `osu!lazer` (via OS association / `xdg-open` / Flatpak), in `osu!stable` (directly executing `osu!.exe` on Windows or invoking `wine osu!.exe "Z:\path\to\skin.osk"` with the inferred `WINEPREFIX` on Linux), or simply leave the file saved on disk.
