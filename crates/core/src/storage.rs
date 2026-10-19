use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{io_err, CoreResult};
use crate::paths::hashed_storage_path;

const READ_BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB streaming buffer

/// Computes the SHA-256 digest of a file in a streaming manner and returns a
/// lowercase hex string matching osu!lazer's content-addressed storage format.
pub fn sha256_hex(path: &Path) -> CoreResult<String> {
    let file = File::open(path).map_err(io_err(path))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; READ_BUFFER_SIZE];

    loop {
        let read = reader.read(&mut buffer).map_err(io_err(path))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex_lowercase(&hasher.finalize()))
}

fn hex_lowercase(bytes: &[u8]) -> String {
    // Equivalent to format!("{:x}", ...) for an arbitrary byte slice.
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Result of storing a resource in osu!lazer's content-addressed storage.
#[derive(Debug, Clone)]
pub struct StoredFile {
    pub hash: String,
    pub destination: PathBuf,
    /// `true` if a file with this SHA-256 hash was already present in storage
    /// (deduplicated without copying again).
    pub already_present: bool,
}

/// Copies `source` into the content-addressed storage directory `files_dir` at
/// `files/<h0>/<h0h1>/<hash>`. If a file with the same SHA-256 hash already exists,
/// it is not copied again.
pub fn store_resource(files_dir: &Path, source: &Path) -> CoreResult<StoredFile> {
    let hash = sha256_hex(source)?;
    let destination = hashed_storage_path(files_dir, &hash);

    if destination.is_file() {
        return Ok(StoredFile {
            hash,
            destination,
            already_present: true,
        });
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
    }

    // Copy to a temporary file alongside the destination and rename atomically
    // so concurrent readers never observe a partially written file.
    let tmp_destination = destination.with_extension("tmp-copy");
    std::fs::copy(source, &tmp_destination).map_err(io_err(source))?;
    std::fs::rename(&tmp_destination, &destination).map_err(io_err(&destination))?;

    Ok(StoredFile {
        hash,
        destination,
        already_present: false,
    })
}

/// Stores a raw byte slice directly in the content-addressed storage directory `files_dir`
/// at `files/<h0>/<h0h1>/<hash>`. Used for storing generated `skin.ini` content.
pub fn store_resource_bytes(files_dir: &Path, bytes: &[u8]) -> CoreResult<StoredFile> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hex_lowercase(&hasher.finalize());
    let destination = hashed_storage_path(files_dir, &hash);

    if destination.is_file() {
        return Ok(StoredFile {
            hash,
            destination,
            already_present: true,
        });
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
    }

    let tmp_destination = destination.with_extension("tmp-copy");
    std::fs::write(&tmp_destination, bytes).map_err(io_err(&tmp_destination))?;
    std::fs::rename(&tmp_destination, &destination).map_err(io_err(&destination))?;

    Ok(StoredFile {
        hash,
        destination,
        already_present: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "osu-mixer-storage-test-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sha256_matches_known_vector() {
        let dir = temp_dir("hash");
        let file_path = dir.join("sample.txt");
        std::fs::write(&file_path, b"abc").unwrap();

        // Standard SHA-256 test vector for "abc".
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let actual = sha256_hex(&file_path).unwrap();
        assert_eq!(actual, expected);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn store_resource_copies_and_dedups() {
        let source_dir = temp_dir("source");
        let files_dir = temp_dir("files-root").join("files");

        let source_file = source_dir.join("cursor.png");
        std::fs::write(&source_file, b"fake png bytes").unwrap();

        let first = store_resource(&files_dir, &source_file).unwrap();
        assert!(!first.already_present);
        assert!(first.destination.is_file());

        let second = store_resource(&files_dir, &source_file).unwrap();
        assert!(second.already_present);
        assert_eq!(first.hash, second.hash);
        assert_eq!(first.destination, second.destination);

        std::fs::remove_dir_all(&source_dir).ok();
        std::fs::remove_dir_all(files_dir.parent().unwrap()).ok();
    }

    #[test]
    fn store_resource_bytes_stores_and_deduplicates() {
        let files_dir = temp_dir("bytes-root").join("files");
        let bytes = b"[General]\nName: Test\n";

        let first = store_resource_bytes(&files_dir, bytes).unwrap();
        assert!(!first.already_present);
        assert!(first.destination.is_file());

        let second = store_resource_bytes(&files_dir, bytes).unwrap();
        assert!(second.already_present);
        assert_eq!(first.hash, second.hash);

        std::fs::remove_dir_all(files_dir.parent().unwrap()).ok();
    }
}
