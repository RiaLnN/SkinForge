# skinforge-core

Core logic and CLI for **SkinForge** — skin mixing engine and Realm database bridge wrapper for osu!lazer.

## Features

- **Path Detection (`paths.rs`)**: Automatic cross-platform detection of osu!lazer data directory (`client.realm` and content-addressed `files/` storage) on Windows and Linux (including Flatpak).
- **Process Guard (`process_guard.rs`)**: Process detection prevents concurrent Realm write conflicts while the game is running.
- **Content-Addressed Storage (`storage.rs`)**: Streaming SHA-256 calculation and deduplicated file placement matching osu!lazer's storage layout (`files/<h0>/<h0h1>/<hash>`).
- **Zero-Copy Mixing (`mixer.rs`)**: Manifest-based skin mixing that references existing file hashes across skins without copying duplicate bytes on disk.
- **Realm Bridge Wrapper (`bridge.rs`)**: Safe interface for the C# dynamic Realm bridge with automatic timestamped `client.realm` backups before any write operation.
- **`skin.ini` Parser & Serializer (`skin_ini.rs`)**: Reads, modifies, and merges configuration sections and combo colours.
- **Component Classification (`categorize.rs`)**: Maps skin files and sprites to canonical slots across Cursor, Gameplay, Audio, and Interface categories.
