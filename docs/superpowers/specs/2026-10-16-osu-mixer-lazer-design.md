# osu! mixer (lazer) — Design Document

**Date:** 2026-10-16  
**Status:** Approved  
**Author:** Pair programming (User & Antigravity)

---

## 1. Intent & Context (The "Why")

In osu! stable, skin mixing tools (such as "osu! Skin Mixer") allowed players to assemble custom skins by combining individual elements (cursors, hitcircles, hitsounds, interface elements) from different installed skins.

In osu! lazer, skins are stored differently:
- Skin metadata and file manifests live in a local Realm database (`client.realm`).
- Resource files (PNG sprites, WAV/MP3 sounds, `skin.ini`) are stored content-addressed by their SHA-256 hash in `<lazer_data>/files/x/xy/<hash>`.

The existing codebase included preliminary stages of a C# `realm-bridge` (reading/writing `client.realm`) and a Rust core (`osu-mixer-core`). However, the previous implementation treated skin donors as external `.osk` zip archives that needed manual extraction and re-hashing. This created redundant disk I/O, duplicate code, and poor UX.

**Goal:** Build a complete, responsive desktop GUI application in Rust (`egui`/`eframe`) that lets users select elements from their existing osu! lazer skins, preview them interactively (visual + audio), and instantly assemble and inject a new mixed skin into osu! lazer via the native C# Realm Bridge with zero-copy file deduplication.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **Realm-Direct Skin Reading:** Automatically detect and list all installed skins in osu! lazer via `realm-bridge dump-skins`.
- **Zero-Copy Mixing:** Leverage osu! lazer's content-addressable storage: substituting a component uses the existing SHA-256 hash of that file already stored in `files/`, without re-copying or duplicating identical assets.
- **Dynamic `skin.ini` Generation & Merging:**
  - Update `Name`, `Author`, and `Combo1..Combo8` colors.
  - Automatically copy component-specific flags from donor skins when substituted (e.g. `CursorExpand`, `CursorRotate`, `HitCircleOverlayAboveNumber`).
  - Serialize to memory, hash via SHA-256, write only this single new resource to `files/`, and register it in the new skin.
- **Curated Component Slots with Fallback & "None":**
  - Curated set of key components matching classic Osu Skin Mixer (Hitcircle, Overlay, Approach Circle, Sliders, Spinner, Cursor, Trail, Middle, Hitnormal, Whistle, Finish, Clap, Combobreak, Section pass/fail, Score numbers, Pause menu).
  - Support explicit "None / Отключить" for elements like `cursormiddle` and `combobreak` via `remove_file`.
  - Fallback: if a selected donor skin lacks the requested component, automatically fall back to the base skin (matching osu!'s native fallback behavior).
- **High-DPI / Variant Hygiene:**
  - When substituting a component, both standard and `@2x` variants are handled.
  - If a base skin has `@2x` files that the donor lacks, `remove_file` is emitted for the orphaned `@2x` variants to prevent osu! lazer from prioritizing old high-DPI sprites.
- **Interactive Live Preview:**
  - Real-time visual preview displaying hitcircle, overlay, animated approach circle, and custom cursor with trail.
  - Real-time hitsound audio playback via `rodio` (normal hit, combobreak).
- **Safe Database Operations:**
  - Verify that osu! lazer is closed before running Realm write transactions (`process_guard`).
  - Create automatic timestamped backups of `client.realm` before applying patches.
  - Filter out protected built-in skins (`Protected == true` or empty files list like *Triangles*, *Argon*, *Classic*) as bases/donors since their assets are embedded in game DLLs.
- **Fast, Lightweight Native GUI:**
  - Built with `egui` and `eframe`.
  - Clean, professional UI using vector icon fonts (Lucide / Phosphor), **strictly no emojis**.

### Non-Goals
- Importing external `.osk` files as the primary mixing mechanism (users can import skins into lazer directly in-game; osu-mixer operates on skins already in lazer).
- Full in-engine gameplay simulator (the preview simulates hitcircle, approach circle, cursor, and hitsound, not an entire replay engine).
- Online skin sharing or repository hosting.

### Anti-Goals
- **No UI Freezing:** Never invoke `realm-bridge`, disk hashing, or audio decoding synchronously on the `egui` UI thread. Background threads and channels (`mpsc`/`crossbeam`) must be used for asynchronous tasks.
- **No Corrupted Realm States:** Never perform partial writes. All database modifications (`duplicate_skin`, `add_file`, `remove_file`) are applied atomically within a single Realm transaction.

---

## 3. UI/UX Design (Variant A: 3-Column Classic Mixer)

The interface follows the classic 3-column layout proven by osu! skin mixers, styled to match the dark theme of osu! lazer:

```
+-----------------------------------------------------------------------------------+
|  [disc] osu! mixer (lazer)          [columns-3 / layout-grid]   [shield] lazer closed|
+--------------------------+--------------------------------+-----------------------+
| 1. Base Skin & Metadata  | 2. Category Slots              | 3. Live Preview & Mix |
|                          | [Cursor][Game][Audio][UI][Col] |                       |
| Base: [ Aristia (Edit) ] |                                | +-------------------+ |
| New Name: [ My Mixed   ] | Cursor Sprite:                 | |   (Approach)      | |
| Author:   [ riann      ] |   [ WhiteCat 2.0 (cursor)    ] | |   [ 1 ] hitcircle | |
|                          | Trail:                         | |   (cursor trail)  | |
| Installed Skins (List):  |   [ Aristia (cursortrail)    ] | +-------------------+ |
| - Aristia (Edit) [Base]  | Middle:                        |                       |
| - WhiteCat 2.0           |   [ None / Отключить         ] | Sound Test:           |
| - Yugen                  |                                | [vol] Hit  [break]    |
| - BTMC Freedom Dive      | [zap] Take all from WhiteCat   |                       |
|                          |                                | Queued Patch Ops:     |
| [x] Auto-backup Realm    | Combo Colors (Skin.ini):       | - duplicate_skin...   |
|                          | [o] #ff66ab  [o] #33bbee       | - add_file cursor.png |
|                          | [o] #88ff44  [o] #ffaa22       | - remove_file middle  |
|                          |                                |                       |
|                          |                                | [sparkles] Create Skin|
+--------------------------+--------------------------------+-----------------------+
```

### Visual Styling & Iconography
- **Icons:** Vector font (e.g. `egui-phosphor` or bundled Lucide TTF) used across buttons, tabs, and headers. **No emojis in the UI**.
- **Palette:** osu! lazer dark violet palette (`#13121a`, `#1e1b29`, `#272338`, `#3b3552`) with lazer pink accent (`#ff66ab`) and status accents (`#38ef7d`, `#ffb703`).

---

## 4. Architecture & Technical Components

### Project Structure (Cargo Workspace)
```
Osu-mixer-lazer/
├── Cargo.toml                  # Workspace [core, gui]
├── core/                       # osu-mixer-core library
│   ├── Cargo.toml
│   └── src/
│       ├── bridge.rs           # RealmBridge wrapper (CLI invocation of C# bridge)
│       ├── categorize.rs       # Skin file categorization rules & component families
│       ├── error.rs            # Typed error handling (thiserror)
│       ├── mixer.rs            # Mixing logic: MixPlan, SkinManifest, Patch generation
│       ├── paths.rs            # Lazer directory discovery & content-addressed paths
│       ├── process_guard.rs    # Game process check (sysinfo)
│       ├── skin_ini.rs         # Skin.ini parser, serializer, merger & combo colour builder
│       └── storage.rs          # Content-addressed hashing & file storing (files/x/xy/<hash>)
├── gui/                        # osu-mixer-gui binary crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # eframe entrypoint & window configuration
│       ├── app.rs              # App state, message handling, update loop
│       ├── views/
│       │   ├── sidebar.rs      # Base skin selection, metadata, skin list
│       │   ├── categories.rs   # Category tabs (Cursor, Gameplay, Audio, Interface, Colors)
│       │   └── preview.rs      # Interactive live preview canvas & sound trigger
│       ├── preview_scene.rs    # Visual composition of sprites (hitcircle + approach + cursor)
│       ├── audio.rs            # Rodio audio player & sound sample cache
│       ├── textures.rs         # Background texture loader & TextureHandle cache
│       └── icons.rs            # Vector icon definitions (Phosphor / Lucide)
└── realm-bridge/               # C# console app using Dynamic Realm API (.NET 8)
    ├── Program.cs
    ├── RealmOperations.cs
    ├── Models.cs
    └── realm-bridge.csproj
```

### Data Flow:
1. **Startup:**
   - GUI discovers osu! lazer data directory via `paths::find_lazer_root()`.
   - Spawns a background thread to call `RealmBridge::dump_skins()`.
   - Filters out protected/empty built-in skins (`Protected == true`).
   - Populates app state with available custom skins.
2. **Selection & Configuration:**
   - User chooses a Base Skin.
   - For each slot, user chooses a donor skin, retains base (or automatic fallback if absent in donor), or chooses "None".
   - Batch buttons allow replacing an entire category in one click.
   - User adjusts combo colors and metadata.
3. **Live Preview:**
   - Resolves active component hashes (donor hash, fallback to base hash).
   - Loads sprites directly from `<lazer_data>/files/x/xy/<hash>`.
   - Decodes images in background into `ColorImage`, uploads to `TextureHandle` on UI thread.
   - Caches audio samples into memory, triggers via `rodio::Sink`.
4. **Build & Apply:**
   - Verifies game is closed (`process_guard`).
   - Creates timestamped backup of `client.realm`.
   - Merges donor `skin.ini` flags (`CursorExpand`, etc.) into base `skin.ini`, applies new name/author and combo colors.
   - Stores new `skin.ini` in `<lazer_data>/files/x/xy/<hash>`.
   - Builds `patch.json`:
     - `duplicate_skin`: clones Base Skin.
     - `add_file`: maps substituted files to existing or new hashes.
     - `remove_file`: prunes orphaned `@2x` files and slots explicitly set to "None".
   - Calls `realm-bridge apply-patch --db <client.realm> --patch <patch.json>`.
   - Notifies user of completion.

---

## 5. Builder Calls Summary
- **High-DPI / `@2x` Pruning:** If a donor only has 1x sprites, base `@2x` sprites for that component are pruned via `remove_file` so osu! lazer does not shadow the substituted 1x sprite.
- **Built-in Skins Filtered:** Skins with `Protected = true` (Argon, Triangles, etc.) cannot be duplicated directly from disk because their assets are compiled into game DLLs. They are filtered from the skin list.
- **`skin.ini` Flag Merging:** When substituting Cursor or Hitcircles, relevant rendering flags (`CursorExpand`, `CursorRotate`, `HitCircleOverlayAboveNumber`) from donor `skin.ini` are merged into the final `skin.ini` to guarantee visual fidelity.
