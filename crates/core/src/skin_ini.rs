use std::path::Path;

use crate::error::{io_err, CoreResult};

/// A single `[Name]` section with `Key: Value` pairs in original order
/// (order matters for `Combo1..ComboN` to preserve combo colour order).
#[derive(Debug, Clone, Default)]
pub struct IniSection {
    pub name: String,
    pub entries: Vec<(String, String)>,
}

impl IniSection {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key_str = key.into();
        let val_str = value.into();
        if let Some((_, v)) = self
            .entries
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(&key_str))
        {
            *v = val_str;
        } else {
            self.entries.push((key_str, val_str));
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|(k, _)| !k.eq_ignore_ascii_case(key));
    }
}

#[derive(Debug, Clone, Default)]
pub struct SkinIni {
    pub sections: Vec<IniSection>,
}

impl SkinIni {
    /// Reads and parses `skin.ini` at the specified path. Legacy skin.ini files
    /// are sometimes encoded in ISO-8859-1 / Windows-1251 without BOM rather than
    /// strict UTF-8, so we read raw bytes and use lossy conversion instead of
    /// `fs::read_to_string`.
    pub fn read_from_file(path: &Path) -> CoreResult<Self> {
        let bytes = std::fs::read(path).map_err(io_err(path))?;
        Ok(Self::parse(&String::from_utf8_lossy(&bytes)))
    }

    pub fn parse(text: &str) -> Self {
        let mut sections: Vec<IniSection> = Vec::new();
        let mut current: Option<IniSection> = None;

        for raw_line in text.lines() {
            // Strip UTF-8 BOM on the first line if present.
            let line = raw_line.trim_start_matches('\u{feff}').trim();

            if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                if let Some(section) = current.take() {
                    sections.push(section);
                }
                current = Some(IniSection {
                    name: line[1..line.len() - 1].trim().to_string(),
                    entries: Vec::new(),
                });
                continue;
            }

            // skin.ini uses "Key: Value" pairs (split on the first ':' and trim both sides).
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                if let Some(section) = current.as_mut() {
                    section.entries.push((key, value));
                }
                // Key-value lines outside of any section are ignored safely.
            }
        }

        if let Some(section) = current.take() {
            sections.push(section);
        }

        Self { sections }
    }

    pub fn section(&self, name: &str) -> Option<&IniSection> {
        self.sections
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn general(&self, key: &str) -> Option<&str> {
        self.section("General").and_then(|s| s.get(key))
    }

    pub fn skin_name(&self) -> Option<&str> {
        self.general("Name")
    }

    pub fn skin_author(&self) -> Option<&str> {
        self.general("Author")
    }

    /// Combo colours from `[Colours]`, ordered by `Combo1..ComboN`, as `(index, (r,g,b))`.
    /// Non-combo keys in `[Colours]` (such as `SliderBorder` or `SliderTrackOverride`)
    /// are ignored here.
    pub fn combo_colours(&self) -> Vec<(u32, (u8, u8, u8))> {
        let Some(section) = self.section("Colours") else {
            return Vec::new();
        };

        let mut out: Vec<(u32, (u8, u8, u8))> = section
            .entries
            .iter()
            .filter_map(|(key, value)| {
                let lower = key.to_ascii_lowercase();
                let index_str = lower.strip_prefix("combo")?;
                let index: u32 = index_str.parse().ok()?;
                let rgb = parse_rgb(value)?;
                Some((index, rgb))
            })
            .collect();

        out.sort_by_key(|(index, _)| *index);
        out
    }

    /// Reads an arbitrary RGB key from `[Colours]` (e.g. `SliderBorder`).
    pub fn colour_key(&self, key: &str) -> Option<(u8, u8, u8)> {
        self.section("Colours")
            .and_then(|s| s.get(key))
            .and_then(parse_rgb)
    }

    pub fn set_colour_key(&mut self, key: &str, val: Option<(u8, u8, u8)>) {
        if let Some((r, g, b)) = val {
            self.get_or_create_section_mut("Colours")
                .set(key, format!("{r},{g},{b}"));
        } else if let Some(sec) = self.section_mut("Colours") {
            sec.remove(key);
        }
    }

    fn bool_key(&self, section: &str, key: &str) -> Option<bool> {
        let val = self.section(section)?.get(key)?;
        match val.trim() {
            "1" | "true" | "True" => Some(true),
            "0" | "false" | "False" => Some(false),
            _ => None,
        }
    }

    fn set_bool_key(&mut self, section: &str, key: &str, val: Option<bool>) {
        if let Some(b) = val {
            self.get_or_create_section_mut(section)
                .set(key, if b { "1" } else { "0" });
        } else if let Some(sec) = self.section_mut(section) {
            sec.remove(key);
        }
    }

    fn i32_key(&self, section: &str, key: &str) -> Option<i32> {
        self.section(section)?.get(key)?.trim().parse().ok()
    }

    fn set_i32_key(&mut self, section: &str, key: &str, val: Option<i32>) {
        if let Some(v) = val {
            self.get_or_create_section_mut(section)
                .set(key, v.to_string());
        } else if let Some(sec) = self.section_mut(section) {
            sec.remove(key);
        }
    }

    // Extended [Colours]
    pub fn slider_track_override(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("SliderTrackOverride")
    }

    pub fn set_slider_track_override(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("SliderTrackOverride", val);
    }

    pub fn slider_border(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("SliderBorder")
    }

    pub fn set_slider_border(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("SliderBorder", val);
    }

    pub fn slider_ball(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("SliderBall")
    }

    pub fn set_slider_ball(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("SliderBall", val);
    }

    pub fn menu_glow(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("MenuGlow")
    }

    pub fn set_menu_glow(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("MenuGlow", val);
    }

    pub fn spinner_approach_circle(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("SpinnerApproachCircle")
    }

    pub fn set_spinner_approach_circle(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("SpinnerApproachCircle", val);
    }

    pub fn song_select_active_text(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("SongSelectActiveText")
    }

    pub fn set_song_select_active_text(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("SongSelectActiveText", val);
    }

    pub fn song_select_inactive_text(&self) -> Option<(u8, u8, u8)> {
        self.colour_key("SongSelectInactiveText")
    }

    pub fn set_song_select_inactive_text(&mut self, val: Option<(u8, u8, u8)>) {
        self.set_colour_key("SongSelectInactiveText", val);
    }

    // Extended [General]
    pub fn cursor_rotate(&self) -> Option<bool> {
        self.bool_key("General", "CursorRotate")
    }

    pub fn set_cursor_rotate(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "CursorRotate", val);
    }

    pub fn cursor_expand(&self) -> Option<bool> {
        self.bool_key("General", "CursorExpand")
    }

    pub fn set_cursor_expand(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "CursorExpand", val);
    }

    pub fn cursor_trail_rotate(&self) -> Option<bool> {
        self.bool_key("General", "CursorTrailRotate")
    }

    pub fn set_cursor_trail_rotate(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "CursorTrailRotate", val);
    }

    pub fn cursor_center(&self) -> Option<bool> {
        self.bool_key("General", "CursorCenter")
    }

    pub fn set_cursor_center(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "CursorCenter", val);
    }

    pub fn hit_circle_overlay_above_number(&self) -> Option<bool> {
        self.bool_key("General", "HitCircleOverlayAboveNumber")
    }

    pub fn set_hit_circle_overlay_above_number(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "HitCircleOverlayAboveNumber", val);
    }

    pub fn allow_slider_ball_tint(&self) -> Option<bool> {
        self.bool_key("General", "AllowSliderBallTint")
    }

    pub fn set_allow_slider_ball_tint(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "AllowSliderBallTint", val);
    }

    pub fn animation_framerate(&self) -> Option<i32> {
        self.i32_key("General", "AnimationFramerate")
    }

    pub fn set_animation_framerate(&mut self, val: Option<i32>) {
        self.set_i32_key("General", "AnimationFramerate", val);
    }

    pub fn layered_hit_sounds(&self) -> Option<bool> {
        self.bool_key("General", "LayeredHitSounds")
    }

    pub fn set_layered_hit_sounds(&mut self, val: Option<bool>) {
        self.set_bool_key("General", "LayeredHitSounds", val);
    }

    // Extended [Fonts]
    pub fn hit_circle_overlap(&self) -> Option<i32> {
        self.i32_key("Fonts", "HitCircleOverlap")
    }

    pub fn set_hit_circle_overlap(&mut self, val: Option<i32>) {
        self.set_i32_key("Fonts", "HitCircleOverlap", val);
    }

    pub fn score_overlap(&self) -> Option<i32> {
        self.i32_key("Fonts", "ScoreOverlap")
    }

    pub fn set_score_overlap(&mut self, val: Option<i32>) {
        self.set_i32_key("Fonts", "ScoreOverlap", val);
    }

    pub fn combo_overlap(&self) -> Option<i32> {
        self.i32_key("Fonts", "ComboOverlap")
    }

    pub fn set_combo_overlap(&mut self, val: Option<i32>) {
        self.set_i32_key("Fonts", "ComboOverlap", val);
    }

    pub fn fonts(&self) -> &[(String, String)] {
        self.section("Fonts")
            .map(|s| s.entries.as_slice())
            .unwrap_or(&[])
    }

    pub fn section_mut(&mut self, name: &str) -> Option<&mut IniSection> {
        self.sections
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn get_or_create_section_mut(&mut self, name: &str) -> &mut IniSection {
        if let Some(pos) = self
            .sections
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(name))
        {
            &mut self.sections[pos]
        } else {
            self.sections.push(IniSection {
                name: name.to_string(),
                entries: Vec::new(),
            });
            self.sections.last_mut().unwrap()
        }
    }

    pub fn set_general(&mut self, key: &str, value: &str) {
        self.get_or_create_section_mut("General").set(key, value);
    }

    pub fn set_combo_colours(&mut self, combos: &[(u8, u8, u8)]) {
        let section = self.get_or_create_section_mut("Colours");
        section.entries.retain(|(k, _)| {
            let lower = k.to_ascii_lowercase();
            if let Some(num_str) = lower.strip_prefix("combo") {
                num_str.parse::<u32>().is_err()
            } else {
                true
            }
        });
        for (i, &(r, g, b)) in combos.iter().enumerate() {
            let key = format!("Combo{}", i + 1);
            let val = format!("{r},{g},{b}");
            section.entries.push((key, val));
        }
    }

    pub fn merge_section_keys(&mut self, section_name: &str, donor: &SkinIni, keys: &[&str]) {
        let Some(donor_sec) = donor.section(section_name) else {
            return;
        };
        for &key in keys {
            if let Some(val) = donor_sec.get(key) {
                self.get_or_create_section_mut(section_name).set(key, val);
            }
        }
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push('[');
            out.push_str(&section.name);
            out.push_str("]\n");
            for (key, val) in &section.entries {
                out.push_str(key);
                out.push_str(": ");
                out.push_str(val);
                out.push('\n');
            }
        }
        out
    }
}

impl std::fmt::Display for SkinIni {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.serialize())
    }
}

fn parse_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let mut parts = value.split(',').map(|p| p.trim());
    let r: u8 = parts.next()?.parse().ok()?;
    let g: u8 = parts.next()?.parse().ok()?;
    let b: u8 = parts.next()?.parse().ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
// sample skin.ini
[General]
Name: My Mixed Skin
Author: mixer
Version: latest

[Colours]
Combo1: 255,192,203
Combo2 : 200,200,255
SliderBorder: 255,255,255

[Fonts]
HitCirclePrefix: default
HitCircleOverlap: -2
"#;

    #[test]
    fn parses_general_section() {
        let ini = SkinIni::parse(SAMPLE);
        assert_eq!(ini.skin_name(), Some("My Mixed Skin"));
        assert_eq!(ini.skin_author(), Some("mixer"));
    }

    #[test]
    fn parses_combo_colours_in_order_and_skips_non_combo_keys() {
        let ini = SkinIni::parse(SAMPLE);
        let combos = ini.combo_colours();
        assert_eq!(combos, vec![(1, (255, 192, 203)), (2, (200, 200, 255))]);
        assert_eq!(ini.colour_key("SliderBorder"), Some((255, 255, 255)));
    }

    #[test]
    fn parses_fonts_section() {
        let ini = SkinIni::parse(SAMPLE);
        let overlap = ini.fonts().iter().find(|(k, _)| k == "HitCircleOverlap");
        assert_eq!(overlap.map(|(_, v)| v.as_str()), Some("-2"));
    }

    #[test]
    fn handles_bom_and_missing_sections_gracefully() {
        let ini = SkinIni::parse("\u{feff}[General]\nName: X\n");
        assert_eq!(ini.skin_name(), Some("X"));
        assert!(ini.combo_colours().is_empty());
        assert!(ini.section("Nope").is_none());
    }

    #[test]
    fn serializes_skin_ini_back_to_text() {
        let mut ini = SkinIni::parse(SAMPLE);
        ini.set_general("Name", "New Name");
        ini.set_general("Author", "New Author");
        ini.set_combo_colours(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)]);

        let serialized = ini.serialize();
        let reparsed = SkinIni::parse(&serialized);

        assert_eq!(reparsed.skin_name(), Some("New Name"));
        assert_eq!(reparsed.skin_author(), Some("New Author"));
        assert_eq!(
            reparsed.combo_colours(),
            vec![(1, (255, 0, 0)), (2, (0, 255, 0)), (3, (0, 0, 255))]
        );
        assert_eq!(reparsed.colour_key("SliderBorder"), Some((255, 255, 255)));
        let overlap = reparsed
            .fonts()
            .iter()
            .find(|(k, _)| k == "HitCircleOverlap");
        assert_eq!(overlap.map(|(_, v)| v.as_str()), Some("-2"));
    }

    #[test]
    fn merges_component_flags_from_donor() {
        let mut base = SkinIni::parse(SAMPLE);
        let donor_sample = r#"
[General]
Name: Donor
CursorExpand: 0
CursorRotate: 1

[Fonts]
HitCircleOverlayAboveNumber: 0
"#;
        let donor = SkinIni::parse(donor_sample);
        base.merge_section_keys("General", &donor, &["CursorExpand", "CursorRotate"]);
        base.merge_section_keys("Fonts", &donor, &["HitCircleOverlayAboveNumber"]);

        assert_eq!(
            base.section("General").unwrap().get("CursorExpand"),
            Some("0")
        );
        assert_eq!(
            base.section("General").unwrap().get("CursorRotate"),
            Some("1")
        );
        assert_eq!(
            base.section("Fonts")
                .unwrap()
                .get("HitCircleOverlayAboveNumber"),
            Some("0")
        );
        assert_eq!(base.skin_name(), Some("My Mixed Skin"));
    }

    #[test]
    fn extended_colours_roundtrip() {
        let mut ini = SkinIni::default();
        assert_eq!(ini.slider_track_override(), None);
        ini.set_slider_track_override(Some((30, 40, 50)));
        ini.set_slider_border(Some((200, 210, 220)));
        ini.set_menu_glow(Some((10, 20, 30)));
        assert_eq!(ini.slider_track_override(), Some((30, 40, 50)));
        assert_eq!(ini.slider_border(), Some((200, 210, 220)));
        assert_eq!(ini.menu_glow(), Some((10, 20, 30)));

        let serialized = ini.serialize();
        let reparsed = SkinIni::parse(&serialized);
        assert_eq!(reparsed.slider_track_override(), Some((30, 40, 50)));
        assert_eq!(reparsed.slider_border(), Some((200, 210, 220)));
        assert_eq!(reparsed.menu_glow(), Some((10, 20, 30)));

        ini.set_slider_track_override(None);
        assert_eq!(ini.slider_track_override(), None);
    }

    #[test]
    fn extended_general_and_fonts_roundtrip() {
        let mut ini = SkinIni::default();
        ini.set_cursor_rotate(Some(true));
        ini.set_cursor_expand(Some(false));
        ini.set_animation_framerate(Some(60));
        ini.set_hit_circle_overlay_above_number(Some(true));
        ini.set_hit_circle_overlap(Some(3));
        ini.set_score_overlap(Some(2));

        let serialized = ini.serialize();
        let reparsed = SkinIni::parse(&serialized);
        assert_eq!(reparsed.cursor_rotate(), Some(true));
        assert_eq!(reparsed.cursor_expand(), Some(false));
        assert_eq!(reparsed.animation_framerate(), Some(60));
        assert_eq!(reparsed.hit_circle_overlay_above_number(), Some(true));
        assert_eq!(reparsed.hit_circle_overlap(), Some(3));
        assert_eq!(reparsed.score_overlap(), Some(2));
    }
}
