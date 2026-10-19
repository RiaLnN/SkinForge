use serde::{Deserialize, Serialize};

/// Category of a skin component — corresponds to top-level UI tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentCategory {
    Cursor,
    Gameplay,
    Hud,
    Interface,
    Audio,
    /// Uncategorized or misc file preserved during skin cloning.
    Other,
}

/// Collapsible sub-group within a top-level `ComponentCategory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentSubgroup {
    // Cursor
    CursorElements,
    // Gameplay
    HitCircles,
    HitJudgements,
    ComboNumbers,
    Sliders,
    Spinners,
    // Hud & Playfield
    ScorebarHealth,
    ScoreAndInput,
    LazerHud,
    // Interface (Menus & Screens)
    MenuSongSelect,
    RankingResults,
    PauseFail,
    ModIcons,
    // Audio
    GameplayAudio,
    MenuAudio,
}

impl ComponentSubgroup {
    pub fn category(&self) -> ComponentCategory {
        match self {
            Self::CursorElements => ComponentCategory::Cursor,
            Self::HitCircles
            | Self::HitJudgements
            | Self::ComboNumbers
            | Self::Sliders
            | Self::Spinners => ComponentCategory::Gameplay,
            Self::ScorebarHealth | Self::ScoreAndInput | Self::LazerHud => ComponentCategory::Hud,
            Self::MenuSongSelect | Self::RankingResults | Self::PauseFail | Self::ModIcons => {
                ComponentCategory::Interface
            }
            Self::GameplayAudio | Self::MenuAudio => ComponentCategory::Audio,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CursorElements => "Cursor & Trail Elements",
            Self::HitCircles => "Hit Circles & Approach Circles",
            Self::HitJudgements => "Hit Judgements (300 / 100 / 50 / Miss)",
            Self::ComboNumbers => "Combo Numbers (Inside Circles)",
            Self::Sliders => "Sliders (Ball, Follow Circle, Track)",
            Self::Spinners => "Spinners",
            Self::ScorebarHealth => "Scorebar & Health Bar",
            Self::ScoreAndInput => "Score Digits & Input Overlay",
            Self::LazerHud => "osu!lazer HUD Layout",
            Self::MenuSongSelect => "Menu & Song Select Elements",
            Self::RankingResults => "Ranking & Result Screens",
            Self::PauseFail => "Pause & Fail Screens",
            Self::ModIcons => "Mod Icons",
            Self::GameplayAudio => "Gameplay Hitsounds",
            Self::MenuAudio => "Menu & Result Sounds",
        }
    }

    pub fn subgroups_for_category(cat: ComponentCategory) -> &'static [ComponentSubgroup] {
        match cat {
            ComponentCategory::Cursor => &[Self::CursorElements],
            ComponentCategory::Gameplay => &[
                Self::HitCircles,
                Self::HitJudgements,
                Self::ComboNumbers,
                Self::Sliders,
                Self::Spinners,
            ],
            ComponentCategory::Hud => &[Self::ScorebarHealth, Self::ScoreAndInput, Self::LazerHud],
            ComponentCategory::Interface => &[
                Self::MenuSongSelect,
                Self::RankingResults,
                Self::PauseFail,
                Self::ModIcons,
            ],
            ComponentCategory::Audio => &[Self::GameplayAudio, Self::MenuAudio],
            ComponentCategory::Other => &[],
        }
    }

    pub fn default_expanded(&self) -> bool {
        matches!(
            self,
            Self::CursorElements
                | Self::HitCircles
                | Self::HitJudgements
                | Self::ScorebarHealth
                | Self::MenuSongSelect
                | Self::GameplayAudio
        )
    }

    pub fn slots(&self) -> Vec<ComponentSlot> {
        ComponentSlot::all()
            .iter()
            .copied()
            .filter(|s| s.subgroup() == *self)
            .collect()
    }
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
    // Gameplay: Hit Circles & Approach Circles
    Hitcircle,
    HitcircleOverlay,
    ApproachCircle,
    SliderStartCircle,
    SliderStartCircleOverlay,
    // Gameplay: Hit Judgements
    Hit300,
    Hit100,
    Hit50,
    Hit0,
    Hit300g,
    Hit300k,
    // Gameplay: Combo Numbers
    ComboNumbers,
    // Gameplay: Sliders
    SliderBall,
    SliderFollowCircle,
    SliderTrack,
    ReverseArrow,
    // Gameplay: Spinners
    SpinnerBackground,
    SpinnerCircle,
    SpinnerOsu,
    SpinnerMetre,
    SpinnerRpm,
    SpinnerClear,
    // HUD & Playfield: Scorebar & Health Bar
    ScorebarBg,
    ScorebarColour,
    ScorebarKi,
    // HUD & Playfield: Score & Input Overlay
    ScoreNumbers,
    InputOverlayBackground,
    InputOverlayKey,
    // HUD & Playfield: osu!lazer HUD
    HudLayout,
    // Interface: Menu & Song Select
    MenuBackground,
    SongSelectTop,
    SelectionMode,
    SelectionOptions,
    RankPanel,
    // Interface: Ranking & Results
    RankingPanel,
    RankingTitle,
    RankingAccuracy,
    RankingGrades,
    // Interface: Pause & Fail
    PauseOverlay,
    FailBackground,
    PlayUnpause,
    // Interface: Mods
    SelectionMods,
    // Audio: Gameplay Hitsounds
    Hitnormal,
    Hitwhistle,
    Hitfinish,
    Hitclap,
    Combobreak,
    SectionPass,
    SectionFail,
    // Audio: Menu & Result Sounds
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
            // Cursor
            ComponentSlot::Cursor,
            ComponentSlot::CursorTrail,
            ComponentSlot::CursorMiddle,
            ComponentSlot::CursorSmoke,
            // Hit Circles
            ComponentSlot::Hitcircle,
            ComponentSlot::HitcircleOverlay,
            ComponentSlot::ApproachCircle,
            ComponentSlot::SliderStartCircle,
            ComponentSlot::SliderStartCircleOverlay,
            // Hit Judgements
            ComponentSlot::Hit300,
            ComponentSlot::Hit100,
            ComponentSlot::Hit50,
            ComponentSlot::Hit0,
            ComponentSlot::Hit300g,
            ComponentSlot::Hit300k,
            // Combo Numbers
            ComponentSlot::ComboNumbers,
            // Sliders
            ComponentSlot::SliderBall,
            ComponentSlot::SliderFollowCircle,
            ComponentSlot::SliderTrack,
            ComponentSlot::ReverseArrow,
            // Spinners
            ComponentSlot::SpinnerBackground,
            ComponentSlot::SpinnerCircle,
            ComponentSlot::SpinnerOsu,
            ComponentSlot::SpinnerMetre,
            ComponentSlot::SpinnerRpm,
            ComponentSlot::SpinnerClear,
            // Scorebar & Health Bar
            ComponentSlot::ScorebarBg,
            ComponentSlot::ScorebarColour,
            ComponentSlot::ScorebarKi,
            // Score & Input Overlay
            ComponentSlot::ScoreNumbers,
            ComponentSlot::InputOverlayBackground,
            ComponentSlot::InputOverlayKey,
            // Lazer HUD
            ComponentSlot::HudLayout,
            // Menu & Song Select
            ComponentSlot::MenuBackground,
            ComponentSlot::SongSelectTop,
            ComponentSlot::SelectionMode,
            ComponentSlot::SelectionOptions,
            ComponentSlot::RankPanel,
            // Ranking & Results
            ComponentSlot::RankingPanel,
            ComponentSlot::RankingTitle,
            ComponentSlot::RankingAccuracy,
            ComponentSlot::RankingGrades,
            // Pause & Fail
            ComponentSlot::PauseOverlay,
            ComponentSlot::FailBackground,
            ComponentSlot::PlayUnpause,
            // Mod Icons
            ComponentSlot::SelectionMods,
            // Gameplay Audio
            ComponentSlot::Hitnormal,
            ComponentSlot::Hitwhistle,
            ComponentSlot::Hitfinish,
            ComponentSlot::Hitclap,
            ComponentSlot::Combobreak,
            ComponentSlot::SectionPass,
            ComponentSlot::SectionFail,
            // Menu Audio
            ComponentSlot::MenuClick,
            ComponentSlot::MenuHit,
            ComponentSlot::MenuBack,
            ComponentSlot::Applause,
            ComponentSlot::FailSound,
            ComponentSlot::PauseLoop,
        ]
    }

    pub fn subgroup(&self) -> ComponentSubgroup {
        match self {
            Self::Cursor | Self::CursorTrail | Self::CursorMiddle | Self::CursorSmoke => {
                ComponentSubgroup::CursorElements
            }
            Self::Hitcircle
            | Self::HitcircleOverlay
            | Self::ApproachCircle
            | Self::SliderStartCircle
            | Self::SliderStartCircleOverlay => ComponentSubgroup::HitCircles,
            Self::Hit300
            | Self::Hit100
            | Self::Hit50
            | Self::Hit0
            | Self::Hit300g
            | Self::Hit300k => ComponentSubgroup::HitJudgements,
            Self::ComboNumbers => ComponentSubgroup::ComboNumbers,
            Self::SliderBall | Self::SliderFollowCircle | Self::SliderTrack | Self::ReverseArrow => {
                ComponentSubgroup::Sliders
            }
            Self::SpinnerBackground
            | Self::SpinnerCircle
            | Self::SpinnerOsu
            | Self::SpinnerMetre
            | Self::SpinnerRpm
            | Self::SpinnerClear => ComponentSubgroup::Spinners,
            Self::ScorebarBg | Self::ScorebarColour | Self::ScorebarKi => {
                ComponentSubgroup::ScorebarHealth
            }
            Self::ScoreNumbers | Self::InputOverlayBackground | Self::InputOverlayKey => {
                ComponentSubgroup::ScoreAndInput
            }
            Self::HudLayout => ComponentSubgroup::LazerHud,
            Self::MenuBackground
            | Self::SongSelectTop
            | Self::SelectionMode
            | Self::SelectionOptions
            | Self::RankPanel => ComponentSubgroup::MenuSongSelect,
            Self::RankingPanel
            | Self::RankingTitle
            | Self::RankingAccuracy
            | Self::RankingGrades => ComponentSubgroup::RankingResults,
            Self::PauseOverlay | Self::FailBackground | Self::PlayUnpause => {
                ComponentSubgroup::PauseFail
            }
            Self::SelectionMods => ComponentSubgroup::ModIcons,
            Self::Hitnormal
            | Self::Hitwhistle
            | Self::Hitfinish
            | Self::Hitclap
            | Self::Combobreak
            | Self::SectionPass
            | Self::SectionFail => ComponentSubgroup::GameplayAudio,
            Self::MenuClick
            | Self::MenuHit
            | Self::MenuBack
            | Self::Applause
            | Self::FailSound
            | Self::PauseLoop => ComponentSubgroup::MenuAudio,
        }
    }

    pub fn category(&self) -> ComponentCategory {
        self.subgroup().category()
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::CursorTrail => "Cursor Trail",
            Self::CursorMiddle => "Cursor Middle",
            Self::CursorSmoke => "Cursor Smoke",
            Self::Hitcircle => "Hit Circle",
            Self::HitcircleOverlay => "Hit Circle Overlay",
            Self::ApproachCircle => "Approach Circle",
            Self::SliderStartCircle => "Slider Start Circle",
            Self::SliderStartCircleOverlay => "Slider Start Overlay",
            Self::Hit300 => "Hit 300",
            Self::Hit100 => "Hit 100",
            Self::Hit50 => "Hit 50",
            Self::Hit0 => "Hit Miss (0)",
            Self::Hit300g => "Hit 300g (Katu)",
            Self::Hit300k => "Hit 300k (Geki)",
            Self::ComboNumbers => "Combo Numbers (0–9)",
            Self::SliderBall => "Slider Ball",
            Self::SliderFollowCircle => "Slider Follow Circle",
            Self::SliderTrack => "Slider Track",
            Self::ReverseArrow => "Reverse Arrow",
            Self::SpinnerBackground => "Spinner Background",
            Self::SpinnerCircle => "Spinner Circle",
            Self::SpinnerOsu => "Spinner Osu!",
            Self::SpinnerMetre => "Spinner Metre",
            Self::SpinnerRpm => "Spinner RPM",
            Self::SpinnerClear => "Spinner Clear",
            Self::ScorebarBg => "Scorebar Background",
            Self::ScorebarColour => "Scorebar Health Fill",
            Self::ScorebarKi => "Scorebar Marker / Ki",
            Self::ScoreNumbers => "Score Numbers (0–9)",
            Self::InputOverlayBackground => "Input Overlay Background",
            Self::InputOverlayKey => "Input Overlay Key",
            Self::HudLayout => "Lazer HUD Layout",
            Self::MenuBackground => "Menu Background",
            Self::SongSelectTop => "Song Select Top Bar",
            Self::SelectionMode => "Selection Mode Button",
            Self::SelectionOptions => "Selection Options Button",
            Self::RankPanel => "Song Select Rank Panel",
            Self::RankingPanel => "Ranking Panel",
            Self::RankingTitle => "Ranking Title",
            Self::RankingAccuracy => "Ranking Accuracy",
            Self::RankingGrades => "Ranking Grades (X/S/A/B/C/D)",
            Self::PauseOverlay => "Pause Overlay & Buttons",
            Self::FailBackground => "Fail Screen Background",
            Self::PlayUnpause => "Play Unpause Button",
            Self::SelectionMods => "Mod Icons (All Mods)",
            Self::Hitnormal => "Hit Normal",
            Self::Hitwhistle => "Hit Whistle",
            Self::Hitfinish => "Hit Finish",
            Self::Hitclap => "Hit Clap",
            Self::Combobreak => "Combo Break",
            Self::SectionPass => "Section Pass",
            Self::SectionFail => "Section Fail",
            Self::MenuClick => "Menu Click",
            Self::MenuHit => "Menu Hit",
            Self::MenuBack => "Menu Back",
            Self::Applause => "Applause (Results)",
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
            Self::SliderStartCircle => "sliderstartcircle.png",
            Self::SliderStartCircleOverlay => "sliderstartcircleoverlay.png",
            Self::Hit300 => "hit300.png",
            Self::Hit100 => "hit100.png",
            Self::Hit50 => "hit50.png",
            Self::Hit0 => "hit0.png",
            Self::Hit300g => "hit300g.png",
            Self::Hit300k => "hit300k.png",
            Self::ComboNumbers => "default-1.png",
            Self::SliderBall => "sliderb0.png",
            Self::SliderFollowCircle => "sliderfollowcircle.png",
            Self::SliderTrack => "slidertrack0.png",
            Self::ReverseArrow => "reversearrow.png",
            Self::SpinnerBackground => "spinner-background.png",
            Self::SpinnerCircle => "spinner-circle.png",
            Self::SpinnerOsu => "spinner-osu.png",
            Self::SpinnerMetre => "spinner-metre.png",
            Self::SpinnerRpm => "spinner-rpm.png",
            Self::SpinnerClear => "spinner-clear.png",
            Self::ScorebarBg => "scorebar-bg.png",
            Self::ScorebarColour => "scorebar-colour.png",
            Self::ScorebarKi => "scorebar-ki.png",
            Self::ScoreNumbers => "score-0.png",
            Self::InputOverlayBackground => "inputoverlay-background.png",
            Self::InputOverlayKey => "inputoverlay-key.png",
            Self::HudLayout => "skin.json",
            Self::MenuBackground => "menu-background.jpg",
            Self::SongSelectTop => "songselect-top.png",
            Self::SelectionMode => "selection-mode.png",
            Self::SelectionOptions => "selection-options.png",
            Self::RankPanel => "rank-panel.png",
            Self::RankingPanel => "ranking-panel.png",
            Self::RankingTitle => "ranking-title.png",
            Self::RankingAccuracy => "ranking-accuracy.png",
            Self::RankingGrades => "ranking-x.png",
            Self::PauseOverlay => "pause-overlay.png",
            Self::FailBackground => "fail-background.png",
            Self::PlayUnpause => "play-unpause.png",
            Self::SelectionMods => "selection-mod-hidden.png",
            Self::Hitnormal => "normal-hitnormal.wav",
            Self::Hitwhistle => "normal-hitwhistle.wav",
            Self::Hitfinish => "normal-hitfinish.wav",
            Self::Hitclap => "normal-hitclap.wav",
            Self::Combobreak => "combobreak.wav",
            Self::SectionPass => "sectionpass.wav",
            Self::SectionFail => "sectionfail.wav",
            Self::MenuClick => "menuclick.wav",
            Self::MenuHit => "menuhit.wav",
            Self::MenuBack => "menuback.wav",
            Self::Applause => "applause.wav",
            Self::FailSound => "failsound.wav",
            Self::PauseLoop => "pause-loop.wav",
        }
    }

    pub fn is_audio(&self) -> bool {
        self.category() == ComponentCategory::Audio
    }

    pub fn can_be_none(&self) -> bool {
        matches!(
            self,
            Self::CursorMiddle
                | Self::CursorSmoke
                | Self::CursorTrail
                | Self::SliderStartCircle
                | Self::SliderStartCircleOverlay
                | Self::SliderFollowCircle
                | Self::SliderTrack
                | Self::SpinnerBackground
                | Self::SpinnerOsu
                | Self::SpinnerMetre
                | Self::SpinnerRpm
                | Self::SpinnerClear
                | Self::Hit300
                | Self::Hit100
                | Self::Hit50
                | Self::Hit0
                | Self::Hit300g
                | Self::Hit300k
                | Self::ScorebarBg
                | Self::ScorebarColour
                | Self::ScorebarKi
                | Self::InputOverlayBackground
                | Self::InputOverlayKey
                | Self::HudLayout
                | Self::MenuBackground
                | Self::SongSelectTop
                | Self::PauseOverlay
                | Self::FailBackground
                | Self::Combobreak
                | Self::SectionPass
                | Self::SectionFail
                | Self::MenuClick
                | Self::MenuHit
                | Self::MenuBack
                | Self::Applause
                | Self::FailSound
                | Self::PauseLoop
        )
    }

    pub fn supports_single_file_import(&self) -> bool {
        !matches!(
            self,
            Self::ComboNumbers | Self::ScoreNumbers | Self::RankingGrades | Self::SelectionMods
        )
    }

    /// Checks whether a classified component info matches this slot.
    pub fn matches(&self, info: &ComponentInfo) -> bool {
        let lower = info.filename.to_ascii_lowercase();
        let (_, ext) = split_extension(&lower);
        let is_audio_file = AUDIO_EXTENSIONS.contains(&ext);
        if self.is_audio() && !is_audio_file {
            return false;
        }
        if !self.is_audio() && *self != Self::HudLayout && is_audio_file {
            return false;
        }

        let k = info.component_key.as_str();
        match self {
            Self::Cursor => k == "cursor",
            Self::CursorTrail => k == "cursortrail" || k == "cursor-trail",
            Self::CursorMiddle => k == "cursormiddle" || k == "cursor-middle",
            Self::CursorSmoke => k == "cursor-smoke" || k == "cursorsmoke",
            Self::Hitcircle => k == "hitcircle",
            Self::HitcircleOverlay => k == "hitcircleoverlay",
            Self::ApproachCircle => k == "approachcircle",
            Self::SliderStartCircle => k == "sliderstartcircle",
            Self::SliderStartCircleOverlay => k == "sliderstartcircleoverlay",
            Self::Hit300 => k == "hit300",
            Self::Hit100 => k == "hit100" || k == "hit100k",
            Self::Hit50 => k == "hit50",
            Self::Hit0 => k == "hit0",
            Self::Hit300g => k == "hit300g",
            Self::Hit300k => k == "hit300k",
            Self::ComboNumbers => k.starts_with("default-") || k.starts_with("combo-"),
            Self::SliderBall => k.starts_with("sliderb"),
            Self::SliderFollowCircle => k == "sliderfollowcircle",
            Self::SliderTrack => k == "slidertrack",
            Self::ReverseArrow => k == "reversearrow",
            Self::SpinnerBackground => k == "spinner-background",
            Self::SpinnerCircle => k == "spinner-circle",
            Self::SpinnerOsu => k == "spinner-osu",
            Self::SpinnerMetre => k == "spinner-metre",
            Self::SpinnerRpm => k == "spinner-rpm",
            Self::SpinnerClear => k == "spinner-clear",
            Self::ScorebarBg => k == "scorebar-bg",
            Self::ScorebarColour => k == "scorebar-colour",
            Self::ScorebarKi => {
                k == "scorebar-ki"
                    || k == "scorebar-kiddo"
                    || k == "ki"
                    || k == "scorebar-marker"
            }
            Self::ScoreNumbers => k.starts_with("score-"),
            Self::InputOverlayBackground => k == "inputoverlay-background",
            Self::InputOverlayKey => k == "inputoverlay-key",
            Self::HudLayout => k == "skin",
            Self::MenuBackground => k == "menu-background",
            Self::SongSelectTop => k == "songselect-top",
            Self::SelectionMode => k == "selection-mode",
            Self::SelectionOptions => k == "selection-options",
            Self::RankPanel => k == "rank-panel",
            Self::RankingPanel => k == "ranking-panel",
            Self::RankingTitle => k == "ranking-title",
            Self::RankingAccuracy => k == "ranking-accuracy",
            Self::RankingGrades => matches!(
                k,
                "ranking-xh"
                    | "ranking-x"
                    | "ranking-sh"
                    | "ranking-s"
                    | "ranking-a"
                    | "ranking-b"
                    | "ranking-c"
                    | "ranking-d"
                    | "ranking-xh-small"
                    | "ranking-x-small"
                    | "ranking-sh-small"
                    | "ranking-s-small"
                    | "ranking-a-small"
                    | "ranking-b-small"
                    | "ranking-c-small"
                    | "ranking-d-small"
            ),
            Self::PauseOverlay => k.starts_with("pause-"),
            Self::FailBackground => k == "fail-background",
            Self::PlayUnpause => k == "play-unpause",
            Self::SelectionMods => k.starts_with("selection-mod-"),
            Self::Hitnormal => k == "hitnormal",
            Self::Hitwhistle => k == "hitwhistle",
            Self::Hitfinish => k == "hitfinish",
            Self::Hitclap => k == "hitclap",
            Self::Combobreak => k == "combobreak",
            Self::SectionPass => k == "section-pass" || k == "sectionpass",
            Self::SectionFail => k == "section-fail" || k == "sectionfail",
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
    "sliderstartcircle",
    "sliderstartcircleoverlay",
    "sliderb",
    "sliderfollowcircle",
    "slidertrack",
    "sliderscorepoint",
    "reversearrow",
    "spinner-circle",
    "spinner-approachcircle",
    "spinner-background",
    "spinner-osu",
    "spinner-metre",
    "spinner-rpm",
    "spinner-clear",
    "lighting",
    "particle50",
    "particle100",
    "particle300",
    "hit0",
    "hit50",
    "hit100",
    "hit100k",
    "hit300",
    "hit300g",
    "hit300k",
];

const CURSOR_BASES: &[&str] = &[
    "cursor",
    "cursor-trail",
    "cursortrail",
    "cursormiddle",
    "cursor-middle",
    "cursor-smoke",
    "cursorsmoke",
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
    "section-pass",
    "sectionpass",
    "section-fail",
    "sectionfail",
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
];

const AUDIO_SAMPLESET_PREFIXES: &[&str] = &["normal-", "soft-", "drum-"];

const HUD_BASES: &[&str] = &[
    "scorebar-bg",
    "scorebar-colour",
    "scorebar-marker",
    "scorebar-ki",
    "scorebar-kiddo",
    "ki",
    "inputoverlay-background",
    "inputoverlay-key",
];

const INTERFACE_BASES: &[&str] = &[
    "section-pass",
    "section-fail",
    "menu-back",
    "menuback",
    "menu-background",
    "songselect-top",
    "selection-mode",
    "selection-options",
    "rank-panel",
    "pause-back",
    "pause-continue",
    "pause-retry",
    "pause-overlay",
    "fail-background",
    "play-unpause",
    "ranking-panel",
    "ranking-graph",
    "ranking-title",
    "ranking-accuracy",
];

const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "ogg"];

pub fn classify(filename: &str) -> ComponentInfo {
    let lower = filename.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or(&lower);

    if basename == "skin.json" {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Hud,
            component_key: "skin".to_string(),
        };
    }

    let (stem, extension) = split_extension(basename);
    let stem = strip_retina_suffix(stem);
    let is_audio_ext = AUDIO_EXTENSIONS.contains(&extension);

    // Numbered element sets where the digit indicates component identity.
    for prefix in ["default-", "combo-"] {
        if let Some(rest) = stem.strip_prefix(prefix) {
            if is_all_digits(rest) {
                return ComponentInfo {
                    filename: filename.to_string(),
                    category: ComponentCategory::Gameplay,
                    component_key: stem.to_string(),
                };
            }
        }
    }

    if let Some(rest) = stem.strip_prefix("score-") {
        if is_all_digits(rest)
            || matches!(rest, "comma" | "dot" | "percent" | "x")
        {
            return ComponentInfo {
                filename: filename.to_string(),
                category: ComponentCategory::Hud,
                component_key: stem.to_string(),
            };
        }
    }

    // Hit judgements with optional animation frame suffix (e.g. hit300-0.png, hit300g-0.png, hit0.png)
    if !is_audio_ext {
        for judge in ["hit300g", "hit300k", "hit300", "hit100k", "hit100", "hit50", "hit0"] {
            if stem == judge {
                return ComponentInfo {
                    filename: filename.to_string(),
                    category: ComponentCategory::Gameplay,
                    component_key: judge.to_string(),
                };
            }
            if let Some(rest) = stem.strip_prefix(judge) {
                let rest = rest.strip_prefix('-').unwrap_or(rest);
                if is_all_digits(rest) {
                    return ComponentInfo {
                        filename: filename.to_string(),
                        category: ComponentCategory::Gameplay,
                        component_key: judge.to_string(),
                    };
                }
            }
        }
    }

    // Mod icons (selection-mod-*)
    if !is_audio_ext && stem.starts_with("selection-mod-") {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Interface,
            component_key: strip_trailing_digits(stem).to_string(),
        };
    }

    // Ranking elements (ranking-panel, ranking-title, ranking-accuracy, ranking-x/s/a/b/c/d)
    if !is_audio_ext && stem.starts_with("ranking-") {
        let key = strip_trailing_digits(stem).trim_end_matches('-');
        let normalized_key = if key.is_empty() { stem } else { key };
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Interface,
            component_key: normalized_key.to_string(),
        };
    }

    for &prefix in &["comma", "dot", "percent"] {
        if !is_audio_ext && stem.starts_with(prefix) {
            return ComponentInfo {
                filename: filename.to_string(),
                category: ComponentCategory::Hud,
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

    if is_audio_ext && AUDIO_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Audio,
            component_key: base_without_variation.to_string(),
        };
    }

    if !is_audio_ext
        && (CURSOR_BASES.contains(&base_without_variation)
            || CURSOR_BASES.contains(&without_sampleset))
    {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Cursor,
            component_key: base_without_variation.to_string(),
        };
    }

    if !is_audio_ext && GAMEPLAY_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Gameplay,
            component_key: base_without_variation.to_string(),
        };
    }

    if !is_audio_ext && HUD_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Hud,
            component_key: base_without_variation.to_string(),
        };
    }

    if !is_audio_ext && INTERFACE_BASES.contains(&base_without_variation) {
        return ComponentInfo {
            filename: filename.to_string(),
            category: ComponentCategory::Interface,
            component_key: base_without_variation.to_string(),
        };
    }

    // Audio file fallback
    if is_audio_ext {
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
    !s.is_empty() && s.chars().all(|c: char| c.is_ascii_digit())
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
    fn classifies_hud_and_interface_elements() {
        assert_eq!(
            key_and_category("scorebar-bg.png"),
            (ComponentCategory::Hud, "scorebar-bg".into())
        );
        assert_eq!(
            key_and_category("section-pass.png"),
            (ComponentCategory::Interface, "section-pass".into())
        );
        assert_eq!(
            key_and_category("score-7.png"),
            (ComponentCategory::Hud, "score-7".into())
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
        assert_eq!(hud.category, ComponentCategory::Hud);
        assert_eq!(hud.component_key, "skin");
        assert!(ComponentSlot::HudLayout.matches(&hud));

        let click = classify("menuclick.wav");
        assert_eq!(click.category, ComponentCategory::Audio);
        assert!(ComponentSlot::MenuClick.matches(&click));

        let applause_ogg = classify("applause.ogg");
        assert_eq!(applause_ogg.category, ComponentCategory::Audio);
        assert!(ComponentSlot::Applause.matches(&applause_ogg));

        let pause_mp3 = classify("pause-loop.mp3");
        assert_eq!(pause_mp3.category, ComponentCategory::Audio);
        assert!(ComponentSlot::PauseLoop.matches(&pause_mp3));
    }

    #[test]
    fn single_file_import_support() {
        assert!(ComponentSlot::Cursor.supports_single_file_import());
        assert!(ComponentSlot::Hitcircle.supports_single_file_import());
        assert!(ComponentSlot::HudLayout.supports_single_file_import());
        assert!(ComponentSlot::Combobreak.supports_single_file_import());
        assert!(ComponentSlot::PauseOverlay.supports_single_file_import());
        assert!(!ComponentSlot::ScoreNumbers.supports_single_file_import());
        assert!(!ComponentSlot::ComboNumbers.supports_single_file_import());
        assert!(!ComponentSlot::RankingGrades.supports_single_file_import());
        assert!(!ComponentSlot::SelectionMods.supports_single_file_import());
    }

    #[test]
    fn classifies_section_pass_and_fail_audio_and_slot_is_audio() {
        let pass_wav = classify("sectionpass.wav");
        assert_eq!(pass_wav.category, ComponentCategory::Audio);
        assert!(ComponentSlot::SectionPass.matches(&pass_wav));

        let pass_hyphen = classify("section-pass.wav");
        assert_eq!(pass_hyphen.category, ComponentCategory::Audio);
        assert!(ComponentSlot::SectionPass.matches(&pass_hyphen));

        let fail_wav = classify("sectionfail.wav");
        assert_eq!(fail_wav.category, ComponentCategory::Audio);
        assert!(ComponentSlot::SectionFail.matches(&fail_wav));

        let fail_hyphen = classify("section-fail.wav");
        assert_eq!(fail_hyphen.category, ComponentCategory::Audio);
        assert!(ComponentSlot::SectionFail.matches(&fail_hyphen));

        // Images remain Interface
        let pass_png = classify("section-pass.png");
        assert_eq!(pass_png.category, ComponentCategory::Interface);

        // is_audio check
        assert!(ComponentSlot::Hitnormal.is_audio());
        assert!(ComponentSlot::Combobreak.is_audio());
        assert!(ComponentSlot::SectionPass.is_audio());
        assert!(ComponentSlot::SectionFail.is_audio());
        assert!(ComponentSlot::MenuClick.is_audio());
        assert!(ComponentSlot::Applause.is_audio());
        assert!(!ComponentSlot::Cursor.is_audio());
        assert!(!ComponentSlot::Hitcircle.is_audio());
        assert!(!ComponentSlot::HudLayout.is_audio());
    }

    #[test]
    fn audio_and_image_slots_do_not_cross_match() {
        assert!(!ComponentSlot::SectionPass.matches_filename("section-pass.png"));
        assert!(ComponentSlot::SectionPass.matches_filename("section-pass.wav"));
        assert!(!ComponentSlot::SectionFail.matches_filename("section-fail.png"));
        assert!(ComponentSlot::SectionFail.matches_filename("section-fail.wav"));

        assert!(!ComponentSlot::MenuBack.matches_filename("menu-back.png"));
        assert!(ComponentSlot::MenuBack.matches_filename("menuback.wav"));

        assert!(!ComponentSlot::PauseOverlay.matches_filename("pause-loop.wav"));
        assert!(ComponentSlot::PauseOverlay.matches_filename("pause-overlay.png"));
    }

    #[test]
    fn classifies_and_matches_all_expanded_stable_slots_and_subgroups() {
        // Hit Circles & Approach Circles
        assert!(ComponentSlot::SliderStartCircle.matches_filename("sliderstartcircle.png"));
        assert!(ComponentSlot::SliderStartCircleOverlay
            .matches_filename("sliderstartcircleoverlay@2x.png"));
        assert_eq!(
            ComponentSlot::SliderStartCircle.subgroup(),
            ComponentSubgroup::HitCircles
        );

        // Combo Numbers
        assert!(ComponentSlot::ComboNumbers.matches_filename("default-1.png"));
        assert!(ComponentSlot::ComboNumbers.matches_filename("default-9@2x.png"));
        assert!(ComponentSlot::ComboNumbers.matches_filename("combo-4.png"));
        assert_eq!(
            ComponentSlot::ComboNumbers.subgroup(),
            ComponentSubgroup::ComboNumbers
        );

        // Sliders
        assert!(ComponentSlot::SliderBall.matches_filename("sliderb.png"));
        assert!(ComponentSlot::SliderBall.matches_filename("sliderb0.png"));
        assert!(ComponentSlot::SliderTrack.matches_filename("slidertrack0.png"));
        assert!(ComponentSlot::SliderTrack.matches_filename("slidertrack.png"));
        assert_eq!(
            ComponentSlot::SliderTrack.subgroup(),
            ComponentSubgroup::Sliders
        );

        // Spinners
        assert!(ComponentSlot::SpinnerBackground.matches_filename("spinner-background.png"));
        assert!(ComponentSlot::SpinnerCircle.matches_filename("spinner-circle.png"));
        assert!(ComponentSlot::SpinnerOsu.matches_filename("spinner-osu.png"));
        assert!(ComponentSlot::SpinnerMetre.matches_filename("spinner-metre.png"));
        assert!(ComponentSlot::SpinnerRpm.matches_filename("spinner-rpm.png"));
        assert!(ComponentSlot::SpinnerClear.matches_filename("spinner-clear.png"));
        assert_eq!(
            ComponentSlot::SpinnerClear.subgroup(),
            ComponentSubgroup::Spinners
        );

        // Hit Judgements
        assert!(ComponentSlot::Hit300.matches_filename("hit300.png"));
        assert!(ComponentSlot::Hit300.matches_filename("hit300-0@2x.png"));
        assert!(!ComponentSlot::Hit300.matches_filename("hit300g.png"));
        assert!(!ComponentSlot::Hit300.matches_filename("hit300k.png"));
        assert!(ComponentSlot::Hit100.matches_filename("hit100.png"));
        assert!(ComponentSlot::Hit50.matches_filename("hit50.png"));
        assert!(ComponentSlot::Hit0.matches_filename("hit0.png"));
        assert!(ComponentSlot::Hit300g.matches_filename("hit300g.png"));
        assert!(ComponentSlot::Hit300k.matches_filename("hit300k.png"));
        assert_eq!(
            ComponentSlot::Hit0.subgroup(),
            ComponentSubgroup::HitJudgements
        );

        // Scorebar & Health Bar (Hud category)
        assert!(ComponentSlot::ScorebarBg.matches_filename("scorebar-bg.png"));
        assert!(ComponentSlot::ScorebarColour.matches_filename("scorebar-colour.png"));
        assert!(ComponentSlot::ScorebarKi.matches_filename("scorebar-kiddo.png"));
        assert!(ComponentSlot::ScorebarKi.matches_filename("scorebar-ki.png"));
        assert!(ComponentSlot::ScorebarKi.matches_filename("ki.png"));
        assert_eq!(ComponentSlot::ScorebarBg.category(), ComponentCategory::Hud);
        assert_eq!(
            ComponentSlot::ScorebarBg.subgroup(),
            ComponentSubgroup::ScorebarHealth
        );

        // Input Overlay
        assert!(ComponentSlot::InputOverlayBackground
            .matches_filename("inputoverlay-background.png"));
        assert!(ComponentSlot::InputOverlayKey.matches_filename("inputoverlay-key.png"));
        assert_eq!(
            ComponentSlot::InputOverlayKey.category(),
            ComponentCategory::Hud
        );

        // Menu & Song Select
        assert!(ComponentSlot::MenuBackground.matches_filename("menu-background.jpg"));
        assert!(ComponentSlot::MenuBackground.matches_filename("menu-background.png"));
        assert!(ComponentSlot::SongSelectTop.matches_filename("songselect-top.png"));
        assert!(ComponentSlot::SelectionMode.matches_filename("selection-mode.png"));
        assert!(ComponentSlot::SelectionOptions.matches_filename("selection-options.png"));
        assert!(ComponentSlot::RankPanel.matches_filename("rank-panel.png"));
        assert_eq!(
            ComponentSlot::MenuBackground.subgroup(),
            ComponentSubgroup::MenuSongSelect
        );

        // Ranking & Result Screens
        assert!(ComponentSlot::RankingPanel.matches_filename("ranking-panel.png"));
        assert!(ComponentSlot::RankingTitle.matches_filename("ranking-title.png"));
        assert!(ComponentSlot::RankingAccuracy.matches_filename("ranking-accuracy.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-x.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-xh@2x.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-s.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-a.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-b.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-c.png"));
        assert!(ComponentSlot::RankingGrades.matches_filename("ranking-d.png"));
        assert!(!ComponentSlot::RankingGrades.matches_filename("ranking-panel.png"));

        // Pause & Fail Screens
        assert!(ComponentSlot::PauseOverlay.matches_filename("pause-overlay.png"));
        assert!(ComponentSlot::FailBackground.matches_filename("fail-background.png"));
        assert!(ComponentSlot::PlayUnpause.matches_filename("play-unpause.png"));

        // Mods
        assert!(ComponentSlot::SelectionMods.matches_filename("selection-mod-hidden.png"));
        assert!(ComponentSlot::SelectionMods.matches_filename("selection-mod-doubletime@2x.png"));
        assert_eq!(
            ComponentSlot::SelectionMods.subgroup(),
            ComponentSubgroup::ModIcons
        );
    }
}
