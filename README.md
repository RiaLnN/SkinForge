<div align="center">

# SkinForge

**Mix osu!lazer and osu!stable skins in a few clicks: combine cursors, hitcircles, judgements, HUD, menus, and hitsounds across both game versions with a live interactive preview.**

[![Latest release](https://img.shields.io/github/v/release/RiaLnN/SkinForge?style=flat-square)](https://github.com/RiaLnN/SkinForge/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey?style=flat-square)

<img src="docs/demo.gif" alt="SkinForge in action: mixing skins and exporting a universal .osk package" width="800">

</div>

---

## Why SkinForge?

In **osu!lazer**, skins live inside a Realm database (`client.realm`) and content-addressed `files/` storage, while in **osu!stable** skins live as folders inside `Skins/`. Mixing elements between the two versions used to mean manually digging through hashed storage or unpacking `.osk` archives by hand.

SkinForge discovers skins from **both `osu!lazer` and `osu!stable`** in a unified list, lets you mix any element across versions, previews the result on an interactive 60 fps canvas, and exports a universal `.osk` skin package ready to open in either game.

## Features

- **Unified `osu!lazer` & `osu!stable` Skin Discovery.** Automatically detects installed skins from `osu!lazer` (`client.realm`) and `osu!stable` (`Skins/`), displays them together with `[lazer]` and `[stable]` badges, and lets you filter by game version or pick custom folders.
- **Deep Component Mixing (42 Slots in Structured Sub-Groups).** Organized into 5 component tabs (**Cursor**, **Gameplay**, **HUD & Playfield**, **Menus & Screens**, **Audio**) plus **Colors & INI**, with collapsible sub-groups for Hit Circles, Combo Numbers, Hit Judgements (`300`/`100`/`50`/`Miss`/`Geki`/`Katu`), Sliders, Spinners, Health/Scorebar, Input Overlay, Song Select, Ranking Screens, Pause/Fail Overlays, Mod Icons, and Hitsounds.
- **Interactive Playfield Preview & Natural Hit Judgements.** Click the hitcircle on the 60 fps canvas as the approach circle closes in to test your cursor, trail, combo tint, health bar, and natural `300` / `100` / `50` / `Miss` hit judgement animations. Hover or click any visual slot row to inspect its sprite and dimensions in the **Selected Element Inspector**.
- **Stateful Audio Audition with Play/Stop & Default Fallback.** Preview any gameplay or menu sound with instant **Play / Stop** toggle controls (essential for long tracks like `applause.wav` or `pause-loop.wav`), a master volume slider, and synthesized default `osu!` sound fallback when a skin omits an audio file.
- **Universal `.osk` Export.** Packages your mixed skin into a standard `.osk` archive compatible with both `osu!lazer` and `osu!stable` — 100% safe to export and open even while the game is running.
- **CLI included** for scripting and inspection.

## Download

1. Open the [**Releases page**](https://github.com/RiaLnN/SkinForge/releases/latest).
2. Download the archive for your system:

| Platform | Archive |
| --- | --- |
| Windows (64-bit) | `skinforge-vX.Y.Z-windows-x64.zip` |
| Linux (64-bit) | `skinforge-vX.Y.Z-linux-x64.tar.gz` |

3. Extract it anywhere. **Keep all files together**, especially the `bridge/` folder: SkinForge uses it at runtime to read `osu!lazer`'s Realm database.

No .NET SDK or Rust toolchain is required.

> **Windows:** SmartScreen may warn about an unrecognized app because the binaries are not code-signed. If you downloaded from the official Releases page, click **More info → Run anyway**.

## Quick start

1. Run `skinforge` (`skinforge.exe` on Windows).
2. Choose a **Base skin** on the left (use the `[lazer]` and `[stable]` toggles or search bar to filter your library, or select custom `lazer` / `stable` folders if installed in a non-default location).
3. Enter a name and creator for your new skin.
4. In the center tabs (**Cursor**, **Gameplay**, **HUD & Playfield**, **Menus & Screens**, **Audio**, **Colors & INI**), expand any sub-group and pick a donor skin or import a custom file from disk (`Folder` button) for each element you want to customize. Use **Category batch** or **Batch sub-group** to swap an entire group in one click.
5. Test your mix in the right panel: click the hitcircle on the canvas to trigger hit judgements (`300`/`100`/`50`/`Miss`), hover any component row to inspect its sprite, and audition sounds with the **Play / Stop** buttons.
6. Click **Export .osk Skin**, choose where to save your `.osk` package, and let SkinForge open it directly in `osu!`.

## Compatibility

| | Status |
| --- | --- |
| osu!lazer | Tested with `2026.1005.0-lazer` (Realm database schema v52) |
| osu!stable | All standard `Skins/` installations on Windows and Linux (Wine / Lutris) |
| Realm .NET SDK | `20.1.0` (Dynamic API) |
| Windows 10/11 (x64) | Supported |
| Linux (x64, including Flatpak) | Supported |
| macOS | Not supported |

SkinForge reads `osu!lazer`'s database using Realm's Dynamic API and scans `osu!stable`'s `Skins/` directory directly. Exporting produces a standard `.osk` archive, so it never modifies `client.realm` or your existing `Skins/` folders.

## Command line

`skinforge-cli` is included for scripting and inspection:

```bash
skinforge-cli check-running                 # is osu! running?
skinforge-cli find-lazer                    # print lazer's root, database and storage paths
skinforge-cli dump-skins \                  # dump installed lazer skins as JSON
  --bridge-binary ./bridge/realm-bridge \
  --db ~/.local/share/osu/client.realm
skinforge-cli inspect-donor /path/to/skin   # list components of a donor skin folder
```

## Troubleshooting

<details>
<summary><b>The Linux GUI does not start</b></summary>

Install the required GTK 3 and graphics libraries:

```bash
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libasound2-dev libgtk-3-dev
```

</details>

<details>
<summary><b>My osu!stable or osu!lazer skins are in a custom drive/folder</b></summary>

Click **`lazer folder…`** or **`stable folder…`** in the left sidebar under **Game Version Filter** and select either your `osu!` installation folder or its `Skins` subfolder. SkinForge will immediately scan and merge those skins into the list.

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

Bug reports, ideas and pull requests are welcome. When reporting a bug, please include your OS, osu! version, SkinForge version and steps to reproduce.
[Issues](https://github.com/RiaLnN/SkinForge/issues).

## Support

If you find SkinForge helpful, you can support its development here:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/rialnn)

## License

[MIT](LICENSE)
