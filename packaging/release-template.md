# SkinForge __TAG__

Mix and customize **osu!lazer** skins with interactive live preview and zero-copy performance.

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
2. Close osu!lazer (so `client.realm` is not locked).
3. Run `skinforge` (`skinforge.exe` on Windows).
4. Choose your base skin, swap cursor, gameplay, audio, or interface components, and configure combo colours.
5. Click **Build skin into osu!lazer**. A timestamped backup of your database is created automatically.
6. Launch osu!lazer — your new mixed skin is ready to play!

## Notes

- Automatic timestamped backup (`client.realm.bak-<timestamp>`) is created before any database modifications.
- Linux GUI requires standard GTK 3 and X11/Wayland libraries.

## Verify your download

SHA-256 checksums are attached as `SHA256SUMS.txt`:

```bash
sha256sum -c SHA256SUMS.txt --ignore-missing
```

## What's changed
