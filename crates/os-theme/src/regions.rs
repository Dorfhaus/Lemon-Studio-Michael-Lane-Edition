// SPDX-License-Identifier: GPL-3.0-or-later
//! Theme-load-only color grading. No allocations or color conversion in frame painters.
use crate::{Color, Palette};
pub const SECTIONS: &[(&str, &str, &str)] = &[
    ("toolbar", "Toolbar & shared controls", "toolbar"),
    ("browser", "Browser", "browser_title"),
    ("frames", "Window frames", "splitter"),
    ("playlist", "Playlist", "picker"),
    ("mixer", "Mixer", "mixer_bg"),
    ("inspector", "Attached mixer inspector", "inspector_panel"),
    ("rack", "Channel Rack", "rack_row_box"),
    ("menus", "Menus & dialogs", "menu_drop"),
    ("piano", "Piano Roll", "pr_row"),
    ("finishes", "Control finishes", "button_pressed"),
    ("notes", "Piano Roll note groups", "note_group_01"),
    ("workspace", "Workspace", ""),
    ("dials", "Shared dial controls", ""),
    ("inserts", "Traditional FL-style mixer view", ""),
];
pub fn section(name: &str) -> &'static str {
    if name.starts_with("insert_knob_") {
        return "Traditional FL-style mixer view";
    }
    if name.starts_with("knob_") {
        return "Shared dial controls";
    }
    if name.starts_with("skin_") {
        return "Mixer";
    }
    match name {
        "workspace" | "watermark" | "version_text" => return "Workspace",
        "clip" => return "Playlist",
        "mixer_plate" => return "Mixer",
        _ => {}
    }
    let mut area = SECTIONS[0].1;
    for &key in Palette::NAMES {
        if let Some(&(_, title, _)) = SECTIONS.iter().find(|(_, _, start)| *start == key) {
            area = title;
        }
        if key == name {
            break;
        }
    }
    area
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub hue: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub contrast: f32,
}
impl Default for Tone {
    fn default() -> Self {
        Self {
            hue: 180.0,
            saturation: 100.0,
            brightness: 100.0,
            contrast: 100.0,
        }
    }
}
impl Tone {
    pub fn active(self) -> bool {
        self != Self::default()
    }
    pub fn color(self, color: Color) -> Color {
        if !self.active() {
            return color;
        }
        let (h, s, l) = color.hsl();
        let l = ((l - 0.5) * self.contrast / 100.0 + 0.5) * self.brightness / 100.0;
        Color::from_hsl_pub(
            h + self.hue - 180.0,
            (s * self.saturation / 100.0).clamp(0.0, 1.0),
            l.clamp(0.0, 1.0),
            color.a,
        )
    }
}
pub fn parse(get: impl Fn(&str) -> Option<String>) -> [Tone; 14] {
    std::array::from_fn(|i| {
        let read = |name: &str, default: f32, max: f32| {
            get(&format!("tone-{}-{name}", SECTIONS[i].0))
                .and_then(|s| s.parse::<f32>().ok())
                .filter(|n| n.is_finite())
                .map(|n| n.clamp(0.0, max))
                .unwrap_or(default)
        };
        Tone {
            hue: read("hue", 180.0, 360.0),
            saturation: read("saturation", 100.0, 200.0),
            brightness: read("brightness", 100.0, 200.0),
            contrast: read("contrast", 100.0, 200.0),
        }
    })
}
pub fn apply(palette: &mut Palette, tones: &[Tone; 14]) {
    if !tones.iter().any(|t| t.active()) {
        return;
    }
    let track = palette.sunken_dark;
    let rim = palette.button_border;
    for &key in Palette::NAMES {
        let area = section(key);
        let i = SECTIONS
            .iter()
            .position(|(_, title, _)| *title == area)
            .expect("known color area");
        let color = palette.get_mut(key).expect("known color");
        *color = tones[i].color(*color);
    }
    if tones[4].active() {
        palette.appearance.mixer = Some(crate::appearance::Mixer::Theme);
    }
    if tones[12].active() {
        palette
            .appearance
            .knob
            .get_or_insert(crate::appearance::Knob::Ring);
        if !palette.appearance.knob_track_custom {
            palette.knob_track = tones[12].color(track);
        }
        if !palette.appearance.knob_rim_custom {
            palette.knob_rim = tones[12].color(rim);
        }
        palette.appearance.knob_track_custom = true;
        palette.appearance.knob_rim_custom = true;
    }
    if tones[13].active() {
        palette.appearance.insert_custom = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Design, FlTheme, design, fl};
    #[test]
    fn regional_adjustments_are_bounded_isolated_and_non_accumulating() {
        for design in Design::ALL {
            let text = design::default_text(design);
            let base = design::palette(design, Some(&FlTheme::parse(&text)));
            for (i, (id, title, _)) in SECTIONS.iter().enumerate() {
                let changed = fl::set_key(
                    &text,
                    &format!("LemonStudioStyle.tone-{id}-saturation"),
                    "0",
                );
                let theme = FlTheme::parse(&changed);
                let palette = design::palette(design, Some(&theme));
                for &key in Palette::NAMES {
                    let color = palette.get(key).unwrap();
                    if section(key) == *title {
                        assert_eq!(color.r, color.g, "{id}: {key}");
                        assert_eq!(color.g, color.b);
                    } else {
                        assert_eq!(Some(color), base.get(key), "{id} changed unrelated {key}");
                    }
                }
                assert_eq!(
                    palette,
                    design::palette(design, Some(&theme)),
                    "must not accumulate grading"
                );
                assert_eq!(theme.area_tones[i].saturation, 0.0);
            }
        }
        let tones = parse(|key| match key {
            "tone-playlist-hue" => Some("999".into()),
            "tone-playlist-saturation" => Some("NaN".into()),
            "tone-playlist-brightness" => Some("-30".into()),
            "tone-playlist-contrast" => Some("inf".into()),
            _ => None,
        });
        assert_eq!(
            tones[3],
            Tone {
                hue: 360.0,
                saturation: 100.0,
                brightness: 0.0,
                contrast: 100.0
            }
        );
    }
    #[test]
    fn grading_changes_custom_colors_without_mutating_saved_values() {
        let text = "LemonStudioColor.picker=#D05030\nLemonStudioStyle.tone-playlist-saturation=0\n";
        let theme = FlTheme::parse(text);
        let p = design::palette(Design::Lemon, Some(&theme));
        assert_eq!(p.picker.r, p.picker.g);
        assert_eq!(theme.color_overrides[0].1, Color::rgb(0xd05030));
        let restored = FlTheme::parse(&fl::remove_key(
            text,
            "LemonStudioStyle.tone-playlist-saturation",
        ));
        assert_eq!(
            design::palette(Design::Lemon, Some(&restored)).picker,
            Color::rgb(0xd05030)
        );
    }
}
