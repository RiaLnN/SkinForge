use serde::{Deserialize, Serialize};

/// Категория элемента скина — соответствует вкладкам будущего GUI (Этап 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentCategory {
    Gameplay,
    Cursor,
    Audio,
    Interface,
    /// Файл распознан как часть скина, но не попал ни в один известный
    /// каталог компонентов. Не отбрасываем такие файлы молча — они всё равно
    /// нужны при клонировании скина целиком, просто GUI не даёт им отдельного
    /// превью/слота замены на Этапе 4.
    Other,
}

/// Результат классификации одного файла ресурса.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// Исходное имя файла как есть (с расширением) — то, что кладётся в
    /// патч как `filename`.
    pub filename: String,
    pub category: ComponentCategory,
    /// Канонический ключ компонента без @2x/расширения/номера сэмпла-вариации
    /// (например "hitnormal" для "normal-hitnormal2.wav", "sliderb" для
    /// "sliderb0.png"). Цифровые ключи, где цифра значима (default-N, score-N),
    /// сохраняются с цифрой.
    pub component_key: String,
}

// Точные базовые имена (после снятия @2x и цифрового суффикса-вариации).
const GAMEPLAY_BASES: &[&str] = &[
    "hitcircle",
    "hitcircleoverlay",
    "approachcircle",
    "sliderb",
    "sliderfollowcircle",
    "sliderscorepoint",
    "reversearrow",
    "spinner-circle",
    "spinner-approachcircle",
    "spinner-background",
    "spinner-metre",
    "lighting",
    "particle50",
    "particle100",
    "particle300",
    "hit0",
    "hit50",
    "hit100",
    "hit300",
];

const CURSOR_BASES: &[&str] = &[
    "cursor",
    "cursor-trail",
    "cursortrail",
    "cursormiddle",
    "cursor-middle",
    "cursor-smoke",
];

// Проверяются ПОСЛЕ снятия sampleset-префикса (normal-/soft-/drum-) и цифры вариации.
const AUDIO_BASES: &[&str] = &[
    "hitnormal",
    "hitclap",
    "hitfinish",
    "hitwhistle",
    "slidertick",
    "sliderslide",
    "sliderwhistle",
    "combobreak",
    "spinnerbonus",
    "spinnerspin",
];

const AUDIO_SAMPLESET_PREFIXES: &[&str] = &["normal-", "soft-", "drum-"];

const INTERFACE_BASES: &[&str] = &[
    "scorebar-bg",
    "scorebar-colour",
    "scorebar-marker",
    "scorebar-ki",
    "section-pass",
    "section-fail",
    "inputoverlay-background",
    "inputoverlay-key",
    "menu-back",
    "pause-back",
    "pause-continue",
    "pause-retry",
    "pause-overlay",
    "fail-background",
    "ranking-panel",
    "ranking-graph",
    "ranking-title",
];

const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "ogg"];

pub fn classify(filename: &str) -> ComponentInfo {
    let lower = filename.to_ascii_lowercase();
    let (stem, extension) = split_extension(&lower);
    let stem = strip_retina_suffix(stem);

    // Случаи, где цифра — часть идентичности компонента, а не номер вариации/кадра.
    if let Some(rest) = stem.strip_prefix("default-") {
        if is_all_digits(rest) {
            return ComponentInfo {
                filename: filename.to_string(),
                category: ComponentCategory::Gameplay,
                component_key: stem.to_string(),
            };
        }
    }
    if let Some(rest) = stem.strip_prefix("score-") {
        if is_all_digits(rest) {
            return ComponentInfo {
                filename: filename.to_string(),
                category: ComponentCategory::Interface,
                component_key: stem.to_string(),
            };
        }
    }
    for &prefix in &["ranking-", "comma", "dot", "percent"] {
        if stem.starts_with(prefix) {
            return ComponentInfo {
                filename: filename.to_string(),
                category: ComponentCategory::Interface,
                component_key: stem.to_string(),
            };
        }
    }

    // Снимаем sampleset-префикс для звуков (normal-hitnormal2.wav -> hitnormal2 -> hitnormal).
    let mut without_sampleset = stem;
    for &prefix in AUDIO_SAMPLESET_PREFIXES {
        if let Some(rest) = stem.strip_prefix(prefix) {
            without_sampleset = rest;
            break;
        }
    }

    let base_without_variation = strip_trailing_digits(without_sampleset);

    if CURSOR_BASES.contains(&base_without_variation) || CURSOR_BASES.contains(&without_sampleset) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Cursor,
            component_key: base_without_variation.to_string(),
        };
    }

    if GAMEPLAY_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Gameplay,
            component_key: base_without_variation.to_string(),
        };
    }

    if AUDIO_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Audio,
            component_key: base_without_variation.to_string(),
        };
    }

    if INTERFACE_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Interface,
            component_key: base_without_variation.to_string(),
        };
    }

    // Фолбэк: неизвестное имя, но расширение явно звуковое — размечаем как Audio,
    // чтобы хотя бы категория была осмысленной для GUI (плеер прослушивания и т.п.).
    if AUDIO_EXTENSIONS.contains(&extension) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Audio,
            component_key: without_sampleset.to_string(),
        };
    }

    ComponentInfo {
        filename: filename.to_string(),
        category: ComponentCategory::Other,
        component_key: stem.to_string(),
    }
}

fn split_extension(lower_filename: &str) -> (&str, &str) {
    match lower_filename.rsplit_once('.') {
        Some((stem, ext)) => (stem, ext),
        None => (lower_filename, ""),
    }
}

fn strip_retina_suffix(stem: &str) -> &str {
    stem.strip_suffix("@2x").unwrap_or(stem)
}

fn strip_trailing_digits(s: &str) -> &str {
    s.trim_end_matches(|c: char| c.is_ascii_digit())
}

fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_and_category(filename: &str) -> (ComponentCategory, String) {
        let info = classify(filename);
        (info.category, info.component_key)
    }

    #[test]
    fn classifies_gameplay_elements() {
        assert_eq!(
            key_and_category("hitcircle.png"),
            (ComponentCategory::Gameplay, "hitcircle".into())
        );
        assert_eq!(
            key_and_category("hitcircleoverlay@2x.png"),
            (ComponentCategory::Gameplay, "hitcircleoverlay".into())
        );
        assert_eq!(
            key_and_category("sliderb0.png"),
            (ComponentCategory::Gameplay, "sliderb".into())
        );
        assert_eq!(
            key_and_category("default-3.png"),
            (ComponentCategory::Gameplay, "default-3".into())
        );
        assert_eq!(
            key_and_category("default-3@2x.png"),
            (ComponentCategory::Gameplay, "default-3".into())
        );
    }

    #[test]
    fn classifies_cursor_elements() {
        assert_eq!(
            key_and_category("cursor.png"),
            (ComponentCategory::Cursor, "cursor".into())
        );
        assert_eq!(
            key_and_category("cursor-trail@2x.png"),
            (ComponentCategory::Cursor, "cursor-trail".into())
        );
        assert_eq!(
            key_and_category("cursormiddle.png"),
            (ComponentCategory::Cursor, "cursormiddle".into())
        );
    }

    #[test]
    fn classifies_audio_elements_with_sampleset_prefix_and_variation_number() {
        assert_eq!(
            key_and_category("normal-hitnormal2.wav"),
            (ComponentCategory::Audio, "hitnormal".into())
        );
        assert_eq!(
            key_and_category("soft-hitclap.mp3"),
            (ComponentCategory::Audio, "hitclap".into())
        );
        assert_eq!(
            key_and_category("hitwhistle.ogg"),
            (ComponentCategory::Audio, "hitwhistle".into())
        );
    }

    #[test]
    fn classifies_interface_elements() {
        assert_eq!(
            key_and_category("scorebar-bg.png"),
            (ComponentCategory::Interface, "scorebar-bg".into())
        );
        assert_eq!(
            key_and_category("section-pass.png"),
            (ComponentCategory::Interface, "section-pass".into())
        );
        assert_eq!(
            key_and_category("score-7.png"),
            (ComponentCategory::Interface, "score-7".into())
        );
    }

    #[test]
    fn unknown_audio_extension_falls_back_to_audio_category() {
        assert_eq!(
            key_and_category("some-custom-sound.wav").0,
            ComponentCategory::Audio
        );
    }

    #[test]
    fn truly_unknown_file_is_other() {
        assert_eq!(key_and_category("readme.txt").0, ComponentCategory::Other);
    }
}
