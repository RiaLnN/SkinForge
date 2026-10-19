# SkinForge __TAG__

Mix and customize **osu!lazer** and **osu!stable** skins across both game versions with an interactive 60 fps live preview, stateful audio audition, and universal `.osk` export.

## Downloads

| Platform | File | Contents |
| --- | --- | --- |
| Windows x64 | `__WIN__` | GUI, CLI, Realm bridge, docs |
| Linux x64 | `__LIN__` | GUI, CLI, Realm bridge, docs |

Every archive is self-contained: **no .NET or Rust installation required**.
Archive layout:

```text
skinforge-__TAG__-<platform>/
├── skinforge(.exe)           desktop application
├── skinforge-cli(.exe)       command-line tool
├── bridge/                   self-contained Realm bridge
├── QUICKSTART.txt            short instructions
└── README.md                 full documentation
```

## Quick start

1. Download the archive for your platform and extract it anywhere.
2. Run `skinforge` (`skinforge.exe` on Windows).
3. Choose a base skin from your **osu!lazer** or **osu!stable** library, swap cursor, gameplay, HUD, menu, or audio components across 42 slots, and customize combo colours.
4. Test your mix on the interactive playfield canvas and inspect sprites or audio tracks.
5. Choose whether to auto-open the exported package in **osu!lazer**, **osu!stable** (with automatic Wine support on Linux), or **Save only**, and click **Export .osk Skin**.

## Notes

- Exporting creates a standard `.osk` skin package — 100% safe to use even while `osu!` is running.
- Linux GUI requires standard GTK 3 and X11/Wayland libraries.

## Verify your download

SHA-256 checksums are attached as `SHA256SUMS.txt`:

```bash
sha256sum -c SHA256SUMS.txt --ignore-missing
```

## What's changed
