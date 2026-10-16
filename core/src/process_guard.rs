use crate::error::{CoreError, CoreResult};
use sysinfo::System;

/// Имена процессов, при обнаружении которых работа с базой запрещена.
/// Список сознательно с запасом: и stable, и lazer на разных ОС/сборках
/// (нативный бинарь, Flatpak, AppImage) называются по-разному, а порча
/// realm-базы при параллельной записи — куда дороже, чем один лишний паттерн.
const BLOCKED_PROCESS_NAMES: &[&str] = &["osu!", "osu!.exe", "osu!lazer", "osu!lazer.exe", "osu"];

/// Дополнительно проверяем командную строку на упоминание flatpak app id —
/// имя процесса под flatpak иногда приходит как обёртка (`bwrap`, `flatpak-run`
/// и т.п.), а не как `osu!` напрямую.
const FLATPAK_APP_ID: &str = "sh.ppy.osu";

#[derive(Debug, Clone)]
pub struct RunningGameProcess {
    pub pid: u32,
    pub name: String,
}

/// Возвращает список запущенных процессов игры (stable и/или lazer),
/// не прерывая выполнение — вызывающая сторона решает, что делать.
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

/// Быстрая проверка, запущен ли osu! прямо сейчас.
pub fn is_game_running() -> bool {
    !find_running_game_processes().is_empty()
}

/// Возвращает ошибку, если игра запущена. Вызывать перед любой операцией,
/// затрагивающей `client.realm` (бэкап тоже стоит делать после этой проверки —
/// смысла бэкапить файл, который через секунду может начать переписывать сама игра, нет).
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
        // Не можем детерминированно проверить обнаружение самой игры в CI —
        // просто убеждаемся, что сканирование процессов не падает и возвращает
        // валидный (пусть и пустой) список на машине, где osu! не запущен.
        let result = find_running_game_processes();
        let _ = result.len();
    }

    #[test]
    fn ensure_not_running_ok_when_absent() {
        // На тестовой машине osu! не запущен — вызов не должен возвращать ошибку.
        // Если когда-нибудь тесты переедут на машину, где игра реально запущена,
        // этот тест закономерно упадёт — это и есть проверяемое поведение.
        assert!(ensure_game_not_running().is_ok());
    }
}
