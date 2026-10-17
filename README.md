<div align="center">

# SkinForge

**Mix osu!lazer skins in a few clicks: take the cursor from one skin, hitsounds from another, and build a new skin with a live preview.**

[![Latest release](https://img.shields.io/github/v/release/RiaLnN/SkinForge?style=flat-square)](https://github.com/RiaLnN/SkinForge/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey?style=flat-square)

<img src="docs/demo.gif" alt="SkinForge in action: mixing two skins and building a new one" width="800">

</div>

---

## Why SkinForge?

osu!stable has OsuMixer for combining skins, but **osu!lazer has nothing like it**. In lazer, skins live inside a Realm database (`client.realm`), so mixing them used to mean exporting and re-importing `.osk` files by hand.

SkinForge works with that database directly: pick a base skin, choose donors for individual elements, check the result in the preview, and click **Build**. The new skin shows up in lazer's settings right away.

## Features

- **Mix any element.** Swap the cursor, trail, hitcircle, hitsounds, interface elements and combo colours from different skins. A **Batch replace** dropdown swaps a whole category in one click.
- **Live preview.** A 60 fps canvas shows approach circles, combo colour tinting, hitcircle overlays and cursor movement with trails.
- **Hitsound audition.** Play hitnormal, whistle, finish, clap and combobreak before building.
- **Safe by design.** SkinForge refuses to write while osu! is running and makes a timestamped backup of `client.realm` before every change.
- **No wasted disk space.** New skins reference files lazer already stores, so nothing is copied.
- **CLI included** for scripting and inspection.

## Download

1. Open the [**Releases page**](https://github.com/RiaLnN/SkinForge/releases/latest).
2. Download the archive for your system:

| Platform | Archive |
| --- | --- |
| Windows (64-bit) | `skinforge-vX.Y.Z-windows-x64.zip` |
| Linux (64-bit) | `skinforge-vX.Y.Z-linux-x64.tar.gz` |

3. Extract it anywhere. **Keep all files together**, especially the `bridge/` folder: SkinForge needs it at runtime.

No .NET SDK or Rust toolchain is required.

> **Windows:** SmartScreen may warn about an unrecognized app because the binaries are not code-signed. If you downloaded from the official Releases page, click **More info → Run anyway**.

## Quick start

1. **Close osu!lazer** completely, so `client.realm` is unlocked.
2. Run `skinforge` (`skinforge.exe` on Windows).
3. Choose a **Base skin** on the left.
4. Enter a name and author for the new skin.
5. In the tabs (**Cursor**, **Gameplay**, **Audio**, **Interface**, **Colors**), pick a donor skin for each element you want to change. Choose `Disable / Remove (None)` to drop optional elements such as `cursormiddle` or `combobreak`.
6. Check the result in the right panel: move the mouse over the canvas and click the audio buttons.
7. Click **Build skin into osu!lazer**.
8. Start osu!lazer and select your new skin in settings.

## Compatibility

| | Status |
| --- | --- |
| osu!lazer | Tested with `2026.1005.0-lazer` (Realm database schema v52) |
| Realm .NET SDK | `20.1.0` (Dynamic API) |
| Windows 10/11 (x64) | Supported |
| Linux (x64, including Flatpak) | Supported |
| macOS | Not supported |

SkinForge connects to osu!lazer's database using Realm's Dynamic API, inspecting and patching the schema without compiling against lazer's internal classes. This ensures compatibility across updates as long as the core `Skin` and `File` models remain structurally compatible. If an osu!lazer update ever breaks SkinForge, please [open an issue](https://github.com/RiaLnN/SkinForge/issues) with your lazer version.

## Is it safe?

SkinForge is an **unofficial** tool that edits lazer's database, so it takes precautions:

1. It checks that osu! is closed before any write.
2. It copies `client.realm` (plus `.lock`, `.note` and `.management` if present) to `client.realm.bak-<timestamp>` before changing anything.
3. Changes are applied as an atomic Realm transaction.

These steps make data loss unlikely, but no tool can promise it. Keep your own backup of lazer's data folder if your skins matter to you. To restore, close osu! and copy the backup over `client.realm`.

## Known limitations & roadmap

<!-- TODO: edit to match what you actually plan. -->

- [ ] macOS support
- [ ] Importing `.osk` files as donors from the GUI
- [ ] Code-signed Windows binaries

Ideas and votes are welcome in [Issues](https://github.com/RiaLnN/SkinForge/issues).

## Command line

`skinforge-cli` is included for scripting and inspection:

```bash
skinforge-cli check-running                 # is osu! running?
skinforge-cli find-lazer                    # print lazer's root, database and storage paths
skinforge-cli dump-skins \                  # dump installed skins as JSON
  --bridge-binary ./bridge/realm-bridge \
  --db ~/.local/share/osu/client.realm
skinforge-cli inspect-donor /path/to/skin   # list components of a donor skin folder
```

## Troubleshooting

**"osu! is running" warning won't go away.**
Make sure lazer is fully closed, including background processes and Flatpak wrappers. SkinForge rechecks every 2 seconds and enables the Build button as soon as the game exits.

<details>
<summary><b>The Linux GUI does not start</b></summary>

Install the required GTK 3 and graphics libraries:

```bash
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libasound2-dev libgtk-3-dev
```

</details>

<details>
<summary><b>How does it avoid copying files?</b></summary>

lazer stores files by SHA-256 hash in its `files/` folder. When you take a cursor from skin A into skin B, SkinForge adds a reference to the existing hash in the new skin instead of duplicating the file.

</details>

## For developers

Architecture and repository layout are described in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Prerequisites: [Rust](https://rustup.rs/) (1.80+) and the [.NET 8 SDK](https://dotnet.microsoft.com/download).

```bash
dotnet build realm-bridge/realm-bridge.csproj -c Release   # C# Realm bridge
cargo build --release --workspace                          # Rust workspace

cargo run --release -p skinforge-gui                       # run the GUI
cargo run --release -p skinforge-core -- --help            # run the CLI
```

## Contributing

Bug reports, ideas and pull requests are welcome. When reporting a bug, please include your OS, osu!lazer version, SkinForge version and steps to reproduce.

## License

[MIT](LICENSE)
