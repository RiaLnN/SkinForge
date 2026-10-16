use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::{CoreError, CoreResult};

/// Sets the active skin in osu!lazer's `game.ini` located inside `data_dir`.
///
/// Preserves all existing comments, line breaks, and other keys.
/// Uses an atomic write (temp file + rename) to avoid corrupting `game.ini`.
pub fn set_active_skin(data_dir: &Path, skin_guid: &str) -> CoreResult<()> {
    let ini_path = data_dir.join("game.ini");

    let new_content = if ini_path.is_file() {
        let text = fs::read_to_string(&ini_path)
            .map_err(|e| CoreError::BackupFailed(format!("failed to read game.ini: {e}")))?;

        let mut found = false;
        let mut lines: Vec<String> = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case("skin") {
                    lines.push(format!("Skin = {skin_guid}"));
                    found = true;
                    continue;
                }
            }
            lines.push(line.to_string());
        }

        if !found {
            lines.push(format!("Skin = {skin_guid}"));
        }

        let mut out = lines.join("\n");
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out
    } else {
        format!("[General]\nSkin = {skin_guid}\n")
    };

    let temp_path = data_dir.join(format!(".game.ini.tmp.{}", uuid::Uuid::new_v4()));
    let mut file = fs::File::create(&temp_path)
        .map_err(|e| CoreError::BackupFailed(format!("failed to create temp game.ini: {e}")))?;

    file.write_all(new_content.as_bytes())
        .and_then(|_| file.flush())
        .map_err(|e| CoreError::BackupFailed(format!("failed to write temp game.ini: {e}")))?;

    drop(file);

    fs::rename(&temp_path, &ini_path)
        .map_err(|e| CoreError::BackupFailed(format!("failed to replace game.ini: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn creates_new_game_ini_when_missing() {
        let temp = std::env::temp_dir().join(format!("skinforge-ini-new-{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();

        set_active_skin(&temp, "11111111-2222-3333-4444-555555555555").unwrap();
        let content = fs::read_to_string(temp.join("game.ini")).unwrap();
        assert!(content.contains("Skin = 11111111-2222-3333-4444-555555555555"));

        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn updates_existing_skin_entry_preserving_others() {
        let temp = std::env::temp_dir().join(format!("skinforge-ini-upd-{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        let original =
            "[General]\nRuleset = osu\nSkin = 00000000-0000-0000-0000-000000000000\nVolume = 0.8\n";
        fs::write(temp.join("game.ini"), original).unwrap();

        set_active_skin(&temp, "abcdef01-2345-6789-abcd-ef0123456789").unwrap();
        let content = fs::read_to_string(temp.join("game.ini")).unwrap();
        assert!(content.contains("Skin = abcdef01-2345-6789-abcd-ef0123456789"));
        assert!(content.contains("Ruleset = osu"));
        assert!(content.contains("Volume = 0.8"));
        assert!(!content.contains("00000000-0000-0000-0000-000000000000"));

        fs::remove_dir_all(&temp).ok();
    }
}
