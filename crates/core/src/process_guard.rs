use crate::error::{CoreError, CoreResult};
use sysinfo::System;

/// Process names that block Realm database operations when detected.
/// Broad matching is used across OSes (native, Flatpak, AppImage) to protect
/// Realm databases against corruption from concurrent writes.
const BLOCKED_PROCESS_NAMES: &[&str] = &["osu!", "osu!.exe", "osu!lazer", "osu!lazer.exe", "osu"];

/// Flatpak App ID to check in command lines when process name is wrapped.
const FLATPAK_APP_ID: &str = "sh.ppy.osu";

#[derive(Debug, Clone)]
pub struct RunningGameProcess {
    pub pid: u32,
    pub name: String,
}

/// Returns a list of all currently running game processes (stable and/or lazer).
pub fn find_running_game_processes() -> Vec<RunningGameProcess> {
    let mut system = System::new();
    system.refresh_processes();

    let mut found = Vec::new();

    for (pid, process) in system.processes() {
        let name = process.name().to_string();
        let cmd_line = process.cmd().join(" ");

        let name_matches = BLOCKED_PROCESS_NAMES
            .iter()
            .any(|blocked| name.eq_ignore_ascii_case(blocked));
        let cmdline_matches = cmd_line.contains(FLATPAK_APP_ID);

        if name_matches || cmdline_matches {
            found.push(RunningGameProcess {
                pid: pid.as_u32(),
                name,
            });
        }
    }

    found
}

/// Fast check whether osu! is currently running.
pub fn is_game_running() -> bool {
    !find_running_game_processes().is_empty()
}

/// Returns an error if any game process is currently running.
/// Must be called before any write operation affecting `client.realm`.
pub fn ensure_game_not_running() -> CoreResult<()> {
    if let Some(first) = find_running_game_processes().into_iter().next() {
        return Err(CoreError::GameIsRunning {
            pid: first.pid,
            name: first.name,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_runs_without_panicking() {
        let result = find_running_game_processes();
        let _ = result.len();
    }

    #[test]
    fn ensure_not_running_matches_detection() {
        let running = find_running_game_processes();
        if running.is_empty() {
            assert!(ensure_game_not_running().is_ok());
        } else {
            assert!(ensure_game_not_running().is_err());
        }
    }
}
