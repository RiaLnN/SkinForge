use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{io_err, CoreResult};
use crate::paths::hashed_storage_path;

const READ_BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB — стримим, не грузим файл в память целиком

/// Считает SHA-256 файла потоково (важно для звуков/спрайтов покрупнее — не
/// читаем весь файл в RAM) и возвращает hex-строку в нижнем регистре, как
/// это делает сама lazer.
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
    // Эквивалент format!("{:x}", ...) для произвольного среза, без лишней зависимости.
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Результат помещения ресурса в хранилище lazer.
#[derive(Debug, Clone)]
pub struct StoredFile {
    pub hash: String,
    pub destination: PathBuf,
    /// true, если файл с таким хэшем уже лежал в хранилище (дедуп сработал,
    /// физического копирования не было).
    pub already_present: bool,
}

/// Копирует файл `source` в content-addressed хранилище `files_dir`, кладя
/// его по пути `files/<h0>/<h0h1>/<hash>`. Если файл с таким хэшем уже есть —
/// не копирует повторно (дедупликация, как в самой lazer).
///
/// Хэш и физическое копирование — единственная забота этой функции. Регистрация
/// ссылки на файл в базе (имя ресурса → хэш для конкретного скина) — задача
/// JSON-патча, который потом уходит в realm-bridge (Этап 1).
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

    // Копируем во временный файл рядом и атомарно переименовываем — чтобы
    // параллельный читатель (в теории — сама игра, если вдруг запущена) никогда
    // не увидел частично записанный файл под финальным именем.
    let tmp_destination = destination.with_extension("tmp-copy");
    std::fs::copy(source, &tmp_destination).map_err(io_err(source))?;
    std::fs::rename(&tmp_destination, &destination).map_err(io_err(&destination))?;

    Ok(StoredFile {
        hash,
        destination,
        already_present: false,
    })
}

/// Сохраняет срез байтов напрямую в content-addressed хранилище `files_dir`
/// по пути `files/<h0>/<h0h1>/<hash>`. Применяется для генерации нового `skin.ini`.
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

        // Известный тестовый вектор SHA-256 для строки "abc".
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
