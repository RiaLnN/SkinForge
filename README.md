<div align="center">

# SkinForge

**Mix elements from your osu!lazer and osu!stable skins, preview the result live, and export a ready-to-use `.osk`.**

[![Latest release](https://img.shields.io/github/v/release/RiaLnN/SkinForge?style=flat-square)](https://github.com/RiaLnN/SkinForge/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey?style=flat-square)

<img src="docs/demo.gif" alt="SkinForge in action: mixing osu!lazer and osu!stable skins and exporting a .osk package" width="800">

</div>

---

## Why SkinForge?

osu!stable has OsuMixer, but it only works with stable skins. **osu!lazer has had nothing like it**: its skins live inside a Realm database (`client.realm`), so mixing them meant exporting and unpacking `.osk` archives by hand.

SkinForge lists the skins from **both games in one place**, lets you take any element from any skin (even across versions), shows the result on an interactive preview, and exports a standard `.osk` that either game can import.

## Features

- **Lazer and stable in one list.** Skins from both games appear together with `LAZER` / `STABLE` badges, so you can mix elements across versions.
- **42 elements to swap.** Cursor, hitcircles, judgements, HUD, menus, hitsounds and colours, one by one or a whole group in one click.
- **Live playfield preview.** Click the circles to see `300` / `100` / `50` / `Miss`, and inspect any sprite and its size.
- **Sound audition.** Play and stop any sound; default osu! sounds are used when a skin has none.
- **Safe export.** Produces a standard `.osk` and opens it in lazer or stable (including stable under Wine on Linux). It never edits your game files, and works while the game is running.
- **CLI included** for scripting and inspection.

## Download

1. Open the [**Releases page**](https://github.com/RiaLnN/SkinForge/releases/latest).
2. Download the archive for your system:

| Platform | Archive |
| --- | --- |
| Windows (64-bit) | `skinforge-vX.Y.Z-windows-x64.zip` |
| Linux (64-bit) | `skinforge-vX.Y.Z-linux-x64.tar.gz` |

3. Extract it anywhere. **Keep all files together**, especially the `bridge/` folder: SkinForge uses it to read osu!lazer's skin list.

No .NET SDK or Rust toolchain is required.

> **Windows:** SmartScreen may warn about an unrecognized app because the binaries are not code-signed. If you downloaded from the official Releases page, click **More info → Run anyway**.

## Quick start

1. Run `skinforge` (`skinforge.exe` on Windows). Your lazer and stable skins are detected automatically.
2. Choose a **Base skin** on the left. Use the `lazer` / `stable` buttons or the search bar to filter the list.
3. Enter a name and creator for the new skin.
4. Open the tabs (**Cursor**, **Gameplay**, **HUD & Playfield**, **Menus & Screens**, **Audio**, **Colors & INI**). For each element you want to change, pick a donor skin, or load a file from disk with the **Folder** button. **Category batch** and **Batch sub-group** swap a whole group at once.
5. Test the mix in the right panel: click the hitcircle to see judgements, hover a row to inspect its sprite, and use **Play / Stop** for sounds.
6. Choose what happens after export, then click **Export .osk Skin**:
   - **`osu!lazer`**: opens the `.osk` in lazer, which imports it with its own importer.
   - **`osu!stable`**: opens the `.osk` in stable (under Wine on Linux, the prefix and `Z:\` path are resolved for you).
   - **`Save only`**: just saves the file, so you can import it later or share it.

The result is imported as a new skin; your existing skins are not changed

## Compatibility

| | Status |
| --- | --- |
| osu!lazer | Tested with `2026.1005.0-lazer` |
| osu!stable | Skins are read from standard `Skins/` folders (Windows, and Linux via Wine / Lutris) <!-- TODO: add the stable version you tested --> |
| Windows 10/11 (x64) | Supported |
| Linux (x64, including Flatpak and Wine) | Supported |
| macOS | Not supported |

Exporting produces a standard `.osk` archive, so SkinForge never writes to `client.realm` or to your `Skins/` folders. lazer's database is only read to list your skins.

If an osu!lazer update breaks skin detection, please [open an issue](https://github.com/RiaLnN/SkinForge/issues) with your lazer version.

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
<summary><b>My skins are on a custom drive or folder</b></summary>

Click **`lazer folder…`** or **`stable folder…`** in the left sidebar under **Game Version Filter** and select either your `osu!` installation folder or its `Skins` subfolder. SkinForge scans it and adds those skins to the list.

</details>

<details>
<summary><b>The Linux GUI does not start</b></summary>

Install the required GTK 3 and graphics libraries:

```bash
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libasound2-dev libgtk-3-dev
```

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

Bug reports, ideas and pull requests are welcome on [GitHub Issues](https://github.com/RiaLnN/SkinForge/issues). When reporting a bug, please include your OS, osu! version, SkinForge version and steps to reproduce.

## Support

If you find SkinForge helpful, you can support its development here:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/rialnn)

## License

[MIT](LICENSE)
