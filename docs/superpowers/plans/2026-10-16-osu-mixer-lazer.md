# osu! mixer (lazer) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a complete, responsive native desktop GUI application in Rust (`egui`/`eframe`) that mixes skins directly from osu! lazer's Realm database and storage with real-time visual/audio preview and zero-copy file deduplication.

**Architecture:** A Cargo workspace containing `core` (library for Realm bridge interaction, path resolution, process safety, file categorization, skin.ini serialization, and patch generation) and `gui` (eframe desktop app with 3-column classic mixer layout, Lucide/Phosphor vector icons, background async workers, texture caching, and rodio audio playback). An external native C# CLI binary (`realm-bridge`) performs atomic transactions on `client.realm` via Dynamic API.

**Tech Stack:** Rust (edition 2021, `eframe`, `egui`, `egui-phosphor`, `rodio`, `image`, `sysinfo`, `sha2`, `serde`, `thiserror`), C# (.NET 8 / Native AOT, Realm .NET SDK Dynamic API).

**Spec:** [docs/superpowers/specs/2026-10-16-osu-mixer-lazer-design.md](file:///home/riann/projects/Mix/Osu-mixer-lazer/docs/superpowers/specs/2026-10-16-osu-mixer-lazer-design.md)

---

## Global Constraints

- GUI must be built with `egui` and `eframe`.
- Vector icon fonts (`egui-phosphor` or Lucide) must be used for all UI icons; **strictly no emojis** in the interface.
- Zero-copy mixing: existing files in lazer storage (`<lazer_data>/files/x/xy/<hash>`) must never be re-copied or duplicated on disk; the patch references their existing SHA-256 hashes.
- External `.osk` archives must not be required for the mixing workflow; skins are sourced directly from osu! lazer's `client.realm`.
- Game process safety: `client.realm` write operations must only execute when osu! is confirmed not running, and an automatic timestamped backup must be created before writing.
- Background execution: database calls, file reading/hashing, and texture decoding must happen asynchronously in background threads; the `egui` update loop must never block.
- Atomic Realm transactions: all operations (`duplicate_skin`, `add_file`, `remove_file`) for a mix must apply inside a single Realm write transaction.
- High-DPI hygiene: substituting a component must replace standard and `@2x` variants, and issue `remove_file` for orphaned base `@2x` files.
- Protected skins (`Protected == true`) must be filtered out from selectable skins.

---

## Review Focus

1. **Orphaned `@2x` High-DPI Sprites:** Donor only provides 1x `cursor.png`, but Base skin has `cursor@2x.png`. Expected: patch generates `add_file` for `cursor.png` AND `remove_file` for `cursor@2x.png` so lazer doesn't prefer the old base 2x sprite.
2. **Missing Component Fallback:** User selects a donor for a slot that lacks that file (e.g. donor has no `approachcircle.png`). Expected: mixer falls back cleanly to the base skin's file without error or blank sprite.
3. **Explicit Removal ("None"):** User selects "None" for `cursormiddle` or `combobreak`. Expected: patch generates `remove_file` for those filenames in the new skin.
4. **`skin.ini` Flag Merging:** Donor skin requires `CursorExpand = 0` or `HitCircleOverlayAboveNumber = 0`. Expected: these keys are merged from donor `skin.ini` into the generated `skin.ini`, preserving visual behavior.
5. **UI Responsiveness During Lazer Ops:** User applies a mix or loads many skins. Expected: GUI remains responsive at 60 FPS, displaying spinner/progress bar without freeze.

---

## Tasks

### Task 1: Workspace Scaffolding & Core Skin.ini Serialization / Merging

**Files:**
- Create: `Cargo.toml` (root workspace)
- Modify: `core/Cargo.toml`
- Modify: `core/src/skin_ini.rs`
- Test: `core/src/skin_ini.rs` (unit tests)

- [ ] **Step 1.1: Create root Cargo workspace**
  Configure root `Cargo.toml` with `[workspace] members = ["core", "gui"]`.
  Ensure `core` remains a library crate (`osu_mixer_core`).

- [ ] **Step 1.2: Write failing tests for `skin_ini` serialization and merging**
  In `core/src/skin_ini.rs`, write tests:
  - `serializes_skin_ini_back_to_text`: ensures modified Name, Author, and Combo colours serialize into valid INI text format readable by osu!.
  - `merges_component_flags_from_donor`: ensures `CursorExpand`, `CursorRotate`, and `HitCircleOverlayAboveNumber` are copied from donor INI when present.

- [ ] **Step 1.3: Run tests to confirm failure**
  Run `cargo test -p osu-mixer-core skin_ini` and confirm new tests fail.

- [ ] **Step 1.4: Implement `SkinIni` serialization and merging**
  Implement `SkinIni::serialize(&self) -> String` and `SkinIni::merge_flags(&mut self, donor: &SkinIni, flags: &[&str])`.
  Ensure comments and formatting in non-modified sections are cleanly preserved.

- [ ] **Step 1.5: Run tests to confirm pass**
  Run `cargo test -p osu-mixer-core skin_ini` and ensure all tests pass.

- [ ] **Step 1.6: Commit changes**
  Commit Task 1 changes with message: `feat(core): setup workspace and implement skin.ini serialization and merging`.

---

### Task 2: Core Manifest-Based Mixing Logic (Zero-Copy & Fallback & "None")

**Files:**
- Modify: `core/src/categorize.rs`
- Modify: `core/src/mixer.rs`
- Test: `core/src/mixer.rs` (unit tests)

- [ ] **Step 2.1: Write failing tests for manifest-based mixing**
  In `core/src/mixer.rs`, write unit tests:
  - `mix_plan_generates_zero_copy_references`: when substituting a component from an existing skin, `add_file` uses the existing hash without re-writing files.
  - `mix_plan_removes_orphaned_2x_sprites`: when base has `cursor@2x.png` and donor only has `cursor.png`, emits `remove_file` for `cursor@2x.png`.
  - `mix_plan_handles_explicit_none`: selecting "None" for `cursormiddle.png` emits `remove_file`.
  - `mix_plan_falls_back_when_donor_lacks_component`: when donor lacks the requested slot, retains base file.

- [ ] **Step 2.2: Run tests to confirm failure**
  Run `cargo test -p osu-mixer-core mixer` and confirm failure.

- [ ] **Step 2.3: Implement curated slots and manifest mixing**
  Update `core/src/categorize.rs` with `ComponentSlot` enum representing curated slots:
  - Cursor: `Cursor`, `CursorTrail`, `CursorMiddle`, `CursorSmoke`
  - Gameplay: `Hitcircle`, `HitcircleOverlay`, `ApproachCircle`, `SliderBall`, `SliderFollowCircle`, `ReverseArrow`, `SpinnerCircle`
  - Audio: `Hitnormal`, `Hitwhistle`, `Hitfinish`, `Hitclap`, `Combobreak`, `SectionPass`, `SectionFail`
  - Interface: `ScoreNumbers`, `PauseMenu`, `Combocolours`
  In `core/src/mixer.rs`:
  - Define `SlotSelection { slot: ComponentSlot, source: SlotSource }` where `SlotSource` is `Base | Donor(SkinId) | None`.
  - Implement `resolve_mix_plan(base_skin: &Skin, installed_skins: &[Skin], selections: &[SlotSelection]) -> GeneratedPatch`.

- [ ] **Step 2.4: Run tests to confirm pass**
  Run `cargo test -p osu-mixer-core` and verify all tests pass.

- [ ] **Step 2.5: Commit changes**
  Commit Task 2 changes with message: `feat(core): implement manifest mixing with zero-copy, fallback, and remove_file support`.

---

### Task 3: GUI Crate Scaffolding (`gui/`) & State Machine

**Files:**
- Create: `gui/Cargo.toml`
- Create: `gui/src/main.rs`
- Create: `gui/src/app.rs`
- Create: `gui/src/state.rs`
- Test: `gui/src/state.rs` (unit tests for UI state transitions)

- [ ] **Step 3.1: Initialize `gui` crate with dependencies**
  Configure `gui/Cargo.toml`:
  - `eframe = { version = "0.29", default-features = false, features = ["default_fonts", "glow", "wayland", "x11"] }`
  - `egui = "0.29"`
  - `egui-phosphor = "0.7"` (for clean vector icons, no emojis)
  - `image = { version = "0.25", default-features = false, features = ["png"] }`
  - `rodio = "0.19"`
  - `osu_mixer_core = { path = "../core" }`

- [ ] **Step 3.2: Write tests for UI state management**
  In `gui/src/state.rs`, write tests:
  - `changing_base_skin_updates_default_slots`: when base skin changes, slots update their fallback hashes.
  - `batch_category_assignment_updates_all_slots_in_category`: setting all Cursor components to Donor X updates all cursor slots.
  - `filtering_skins_excludes_protected`: skins with `is_protected: true` are excluded from the selectable skin registry.

- [ ] **Step 3.3: Implement UI state model & background channel**
  In `gui/src/state.rs`:
  - `AppState`: installed skins, selected base skin ID, slot selections map, combo colors, new skin name, creator, status message, async worker channel.
  In `gui/src/app.rs`:
  - Implement `eframe::App` with async message receiver (`try_recv`).
  - Wire font definitions to register `egui-phosphor` icon font.

- [ ] **Step 3.4: Verify state tests and build**
  Run `cargo test -p osu-mixer-gui` and `cargo check -p osu-mixer-gui`.

- [ ] **Step 3.5: Commit changes**
  Commit Task 3 changes with message: `feat(gui): initialize gui crate with state machine and icon setup`.

---

### Task 4: GUI Sidebar & Category Slots Views (Columns 1 & 2)

**Files:**
- Create: `gui/src/views/mod.rs`
- Create: `gui/src/views/sidebar.rs`
- Create: `gui/src/views/categories.rs`
- Modify: `gui/src/app.rs`

- [ ] **Step 4.1: Implement Left Sidebar (Column 1)**
  In `gui/src/views/sidebar.rs`:
  - Base Skin selector (dropdown + search filter).
  - Text input for "Skin Name" (defaults to `<Base> + Mixed`).
  - Text input for "Creator".
  - Scrollable list of installed custom skins with file counts.
  - Checkbox for "Auto-backup client.realm".
  - Game status badge: "osu! lazer is closed (Ready to write)" or "osu! is running (Locked)".

- [ ] **Step 4.2: Implement Category Slots & Tabs (Column 2)**
  In `gui/src/views/categories.rs`:
  - Tab bar: `[Cursor]`, `[Gameplay]`, `[Audio]`, `[Interface]`, `[Colors]`.
  - For each slot row:
    - Component label + target filename badge (e.g. `cursor.png`).
    - Dropdown with available donor skins + `Base (Default)` + `None / Отключить` (where applicable).
    - Status badge: `Base`, `Modified (from Skin X)`, `Removed`.
  - Batch action button at top of each tab: `Take all <Category> from <Donor>`.
  - In `Colors` tab:
    - Color picker widgets for `Combo1..Combo8`.
    - Buttons to add/remove combo colors.

- [ ] **Step 4.3: Integrate into `app.rs` layout**
  Wire sidebar and categories into `egui::CentralPanel` / `egui::SidePanel`.
  Verify layout with `cargo check -p osu-mixer-gui`.

- [ ] **Step 4.4: Commit changes**
  Commit Task 4 changes with message: `feat(gui): implement sidebar and category slot views with batch actions`.

---

### Task 5: Interactive Live Preview & Audio Player (Column 3)

**Files:**
- Create: `gui/src/textures.rs`
- Create: `gui/src/audio.rs`
- Create: `gui/src/views/preview.rs`
- Modify: `gui/src/app.rs`

- [ ] **Step 5.1: Implement texture caching & background loading**
  In `gui/src/textures.rs`:
  - Read sprite file by hash from `<lazer_data>/files/x/xy/<hash>`.
  - Decode PNG bytes in background thread into `egui::ColorImage`.
  - Transfer to UI thread and cache as `egui::TextureHandle` keyed by SHA-256 hash.

- [ ] **Step 5.2: Implement audio player & sample cache**
  In `gui/src/audio.rs`:
  - Initialize `rodio::OutputStream` and reusable `rodio::Sink`.
  - Load audio files (`normal-hitnormal.wav`, `combobreak.wav`, `applause.wav`) by hash from lazer storage.
  - Cache decoded audio in memory and trigger on button click or playfield click.

- [ ] **Step 5.3: Implement Live Preview Canvas (Column 3)**
  In `gui/src/views/preview.rs`:
  - Render dark playfield area with subtle grid.
  - Draw `hitcircle` with active combo color + `hitcircleoverlay` + number `1`.
  - Draw animated `approachcircle` shrinking onto hitcircle.
  - Track mouse position inside preview area to render custom `cursor` + `cursortrail`.
  - Click event triggers `hitnormal` sound + click ripple effect.
  - Sound test buttons: `Hitnormal`, `Break`, `Pass`.
  - Live patch log showing queued operations (`duplicate_skin`, `add_file`, `remove_file`).
  - Action button: `Собрать скин в osu! lazer` (`[sparkles]` icon).

- [ ] **Step 5.4: Test audio and visual scene components**
  Ensure background loading doesn't block UI loop.
  Verify with `cargo check -p osu-mixer-gui`.

- [ ] **Step 5.5: Commit changes**
  Commit Task 5 changes with message: `feat(gui): implement interactive live preview canvas, textures, and rodio audio player`.

---

### Task 6: Build & Apply Pipeline & End-to-End Verification

**Files:**
- Modify: `gui/src/app.rs`
- Create: `gui/src/worker.rs`
- Test: `tests/integration_test.rs`

- [ ] **Step 6.1: Implement background mix worker**
  In `gui/src/worker.rs`:
  - `apply_mix(lazer_root, mix_plan, bridge_binary)`:
    1. Check `process_guard::ensure_game_not_running()`.
    2. Create backup `client.realm.bak.<timestamp>`.
    3. Generate and store modified `skin.ini` in `files/x/xy/<hash>`.
    4. Serialize `patch.json` with `duplicate_skin`, `add_file`, `remove_file`.
    5. Execute `realm-bridge apply-patch --db <client.realm> --patch <patch.json>`.
    6. Return result via channel to UI.

- [ ] **Step 6.2: Implement modal dialogs for progress and result**
  In `gui/src/app.rs`:
  - Show modal overlay during build ("Applying skin mix...").
  - On success: show completion dialog with new skin name.
  - On error: show readable error message with technical details toggle.

- [ ] **Step 6.3: End-to-end integration test**
  In `core/tests/` or workspace test:
  - Mock Realm JSON response from `dump-skins`.
  - Execute full `resolve_mix_plan`.
  - Verify that `patch.json` contains correct operations, zero duplicate file writes, and valid `remove_file` entries.

- [ ] **Step 6.4: Run full workspace test suite**
  Run `cargo test --workspace` and verify all tests pass.

- [ ] **Step 6.5: Commit changes**
  Commit Task 6 changes with message: `feat(gui): implement background apply worker and complete end-to-end flow`.

---

## Plan Self-Review & Verification

Before executing:
- [x] Every task has explicit files listed (Create, Modify, Test).
- [x] Every step follows TDD: write test -> verify failure -> implement -> verify pass.
- [x] No emoji icons; Lucide/Phosphor vector icons specified.
- [x] All builder check decisions incorporated: `@2x` pruning, protected skin filtering, fallback handling, explicit None.
