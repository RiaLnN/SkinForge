use serde::{Deserialize, Serialize};

/// Category of a skin component — corresponds to UI tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentCategory {
    Gameplay,
    Cursor,
    Audio,
    Interface,
    /// Uncategorized or misc file preserved during skin cloning.
    Other,
}

/// Classification outcome for a single skin resource file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// Original file name (with extension) as stored in `SkinFile.filename`.
    pub filename: String,
    pub category: ComponentCategory,
    /// Canonical component key without @2x, extension, or variation numbers
    /// (e.g. "hitnormal" for "normal-hitnormal2.wav", "sliderb" for "sliderb0.png").
    pub component_key: String,
}

/// Canonical curated slots for skin mixing (corresponds to UI slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentSlot {
    // Cursor
    Cursor,
    CursorTrail,
    CursorMiddle,
    CursorSmoke,
    // Gameplay
    Hitcircle,
    HitcircleOverlay,
    ApproachCircle,
    SliderBall,
    SliderFollowCircle,
    ReverseArrow,
    SpinnerCircle,
    // Audio
    Hitnormal,
    Hitwhistle,
    Hitfinish,
    Hitclap,
    Combobreak,
    SectionPass,
    SectionFail,
    // Interface
    ScoreNumbers,
    PauseMenu,
    HudLayout,
    MenuClick,
    MenuHit,
    MenuBack,
    Applause,
    FailSound,
    PauseLoop,
}

impl ComponentSlot {
    pub fn all() -> &'static [ComponentSlot] {
        &[
            ComponentSlot::Cursor,
            ComponentSlot::CursorTrail,
            ComponentSlot::CursorMiddle,
            ComponentSlot::CursorSmoke,
            ComponentSlot::Hitcircle,
            ComponentSlot::HitcircleOverlay,
            ComponentSlot::ApproachCircle,
            ComponentSlot::SliderBall,
            ComponentSlot::SliderFollowCircle,
            ComponentSlot::ReverseArrow,
            ComponentSlot::SpinnerCircle,
            ComponentSlot::Hitnormal,
            ComponentSlot::Hitwhistle,
            ComponentSlot::Hitfinish,
            ComponentSlot::Hitclap,
            ComponentSlot::Combobreak,
            ComponentSlot::SectionPass,
            ComponentSlot::SectionFail,
            ComponentSlot::ScoreNumbers,
            ComponentSlot::PauseMenu,
            ComponentSlot::HudLayout,
            ComponentSlot::MenuClick,
            ComponentSlot::MenuHit,
            ComponentSlot::MenuBack,
            ComponentSlot::Applause,
            ComponentSlot::FailSound,
            ComponentSlot::PauseLoop,
        ]
    }

    pub fn category(&self) -> ComponentCategory {
        match self {
            Self::Cursor | Self::CursorTrail | Self::CursorMiddle | Self::CursorSmoke => {
                ComponentCategory::Cursor
            }
            Self::Hitcircle
            | Self::HitcircleOverlay
            | Self::ApproachCircle
            | Self::SliderBall
            | Self::SliderFollowCircle
            | Self::ReverseArrow
            | Self::SpinnerCircle => ComponentCategory::Gameplay,
            Self::Hitnormal
            | Self::Hitwhistle
            | Self::Hitfinish
            | Self::Hitclap
            | Self::Combobreak
            | Self::SectionPass
            | Self::SectionFail => ComponentCategory::Audio,
            Self::ScoreNumbers
            | Self::PauseMenu
            | Self::HudLayout
            | Self::MenuClick
            | Self::MenuHit
            | Self::MenuBack
            | Self::Applause
            | Self::FailSound
            | Self::PauseLoop => ComponentCategory::Interface,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::CursorTrail => "Cursor Trail",
            Self::CursorMiddle => "Cursor Middle",
            Self::CursorSmoke => "Cursor Smoke",
            Self::Hitcircle => "Hitcircle",
            Self::HitcircleOverlay => "Hitcircle Overlay",
            Self::ApproachCircle => "Approach Circle",
            Self::SliderBall => "Slider Ball",
            Self::SliderFollowCircle => "Slider Follow Circle",
            Self::ReverseArrow => "Reverse Arrow",
            Self::SpinnerCircle => "Spinner Circle",
            Self::Hitnormal => "Hitnormal",
            Self::Hitwhistle => "Hitwhistle",
            Self::Hitfinish => "Hitfinish",
            Self::Hitclap => "Hitclap",
            Self::Combobreak => "Combobreak",
            Self::SectionPass => "Section Pass",
            Self::SectionFail => "Section Fail",
            Self::ScoreNumbers => "Score Numbers",
            Self::PauseMenu => "Pause Menu",
            Self::HudLayout => "HUD Layout",
            Self::MenuClick => "Menu Click",
            Self::MenuHit => "Menu Hit",
            Self::MenuBack => "Menu Back",
            Self::Applause => "Applause",
            Self::FailSound => "Fail Sound",
            Self::PauseLoop => "Pause Loop",
        }
    }

    pub fn primary_filename(&self) -> &'static str {
        match self {
            Self::Cursor => "cursor.png",
            Self::CursorTrail => "cursortrail.png",
            Self::CursorMiddle => "cursormiddle.png",
            Self::CursorSmoke => "cursor-smoke.png",
            Self::Hitcircle => "hitcircle.png",
            Self::HitcircleOverlay => "hitcircleoverlay.png",
            Self::ApproachCircle => "approachcircle.png",
            Self::SliderBall => "sliderb0.png",
            Self::SliderFollowCircle => "sliderfollowcircle.png",
            Self::ReverseArrow => "reversearrow.png",
            Self::SpinnerCircle => "spinner-circle.png",
            Self::Hitnormal => "normal-hitnormal.wav",
            Self::Hitwhistle => "normal-hitwhistle.wav",
            Self::Hitfinish => "normal-hitfinish.wav",
            Self::Hitclap => "normal-hitclap.wav",
            Self::Combobreak => "combobreak.wav",
            Self::SectionPass => "section-pass.png",
            Self::SectionFail => "section-fail.png",
            Self::ScoreNumbers => "score-0.png",
            Self::PauseMenu => "pause-overlay.png",
            Self::HudLayout => "skin.json",
            Self::MenuClick => "menuclick.wav",
            Self::MenuHit => "menuhit.wav",
            Self::MenuBack => "menuback.wav",
            Self::Applause => "applause.wav",
            Self::FailSound => "failsound.wav",
            Self::PauseLoop => "pause-loop.wav",
        }
    }

    pub fn can_be_none(&self) -> bool {
        matches!(
            self,
            Self::CursorMiddle
                | Self::CursorSmoke
                | Self::Combobreak
                | Self::CursorTrail
                | Self::SliderFollowCircle
                | Self::HudLayout
                | Self::MenuClick
                | Self::MenuHit
                | Self::MenuBack
                | Self::Applause
                | Self::FailSound
                | Self::PauseLoop
        )
    }

    pub fn supports_single_file_import(&self) -> bool {
        !matches!(self, Self::ScoreNumbers | Self::PauseMenu)
    }

    /// Checks whether a classified component info matches this slot.
    pub fn matches(&self, info: &ComponentInfo) -> bool {
        let k = info.component_key.as_str();
        match self {
            Self::Cursor => k == "cursor",
            Self::CursorTrail => k == "cursortrail" || k == "cursor-trail",
            Self::CursorMiddle => k == "cursormiddle" || k == "cursor-middle",
            Self::CursorSmoke => k == "cursor-smoke",
            Self::Hitcircle => k == "hitcircle",
            Self::HitcircleOverlay => k == "hitcircleoverlay",
            Self::ApproachCircle => k == "approachcircle",
            Self::SliderBall => k.starts_with("sliderb"),
            Self::SliderFollowCircle => k == "sliderfollowcircle",
            Self::ReverseArrow => k == "reversearrow",
            Self::SpinnerCircle => k == "spinner-circle",
            Self::Hitnormal => k == "hitnormal",
            Self::Hitwhistle => k == "hitwhistle",
            Self::Hitfinish => k == "hitfinish",
            Self::Hitclap => k == "hitclap",
            Self::Combobreak => k == "combobreak",
            Self::SectionPass => k == "section-pass",
            Self::SectionFail => k == "section-fail",
            Self::ScoreNumbers => k.starts_with("score-"),
            Self::PauseMenu => k.starts_with("pause-"),
            Self::HudLayout => k == "skin",
            Self::MenuClick => k == "menuclick" || k == "menu-click",
            Self::MenuHit => k == "menuhit" || k == "menu-hit",
            Self::MenuBack => k == "menuback" || k == "menu-back",
            Self::Applause => k == "applause",
            Self::FailSound => k == "failsound" || k == "fail-sound",
            Self::PauseLoop => k == "pause-loop" || k == "pauseloop",
        }
    }

    /// Checks whether a given filename belongs to this slot.
    pub fn matches_filename(&self, filename: &str) -> bool {
        self.matches(&classify(filename))
    }
}

// Canonical base stems (without @2x and numeric animation suffixes).
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

// Audio base names checked after stripping sampleset prefixes (normal-/soft-/drum-).
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
    "menuback",
    "menuclick",
    "menu-click",
    "menuhit",
    "menu-hit",
    "applause",
    "failsound",
    "fail-sound",
    "pause-loop",
    "pauseloop",
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
    if lower == "skin.json" {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Interface,
            component_key: "skin".to_string(),
        };
    }

    let (stem, extension) = split_extension(&lower);
    let stem = strip_retina_suffix(stem);

    // Numbered element sets where the digit indicates component identity.
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

    // Strip sampleset prefix for audio files (e.g. normal-hitnormal2.wav -> hitnormal).
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

    // Audio file fallback
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

    #[test]
    fn classifies_hud_layout_and_ui_sounds() {
        let hud = classify("skin.json");
        assert_eq!(hud.category, ComponentCategory::Interface);
        assert_eq!(hud.component_key, "skin");
        assert!(ComponentSlot::HudLayout.matches(&hud));

        let click = classify("menuclick.wav");
        assert_eq!(click.category, ComponentCategory::Interface);
        assert!(ComponentSlot::MenuClick.matches(&click));

        let applause_ogg = classify("applause.ogg");
        assert_eq!(applause_ogg.category, ComponentCategory::Interface);
        assert!(ComponentSlot::Applause.matches(&applause_ogg));

        let pause_mp3 = classify("pause-loop.mp3");
        assert_eq!(pause_mp3.category, ComponentCategory::Interface);
        assert!(ComponentSlot::PauseLoop.matches(&pause_mp3));
    }

    #[test]
    fn single_file_import_support() {
        assert!(ComponentSlot::Cursor.supports_single_file_import());
        assert!(ComponentSlot::Hitcircle.supports_single_file_import());
        assert!(ComponentSlot::HudLayout.supports_single_file_import());
        assert!(ComponentSlot::Combobreak.supports_single_file_import());
        assert!(!ComponentSlot::ScoreNumbers.supports_single_file_import());
        assert!(!ComponentSlot::PauseMenu.supports_single_file_import());
    }
}
