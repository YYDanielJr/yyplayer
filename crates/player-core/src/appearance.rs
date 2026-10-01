//! Persisted choices; material tokens live in the renderer, OS observations in platform.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Scheme {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Design {
    Simple,
    #[default]
    Fashion,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Appearance {
    pub scheme: Scheme,
    pub design: Design,
    pub system_accent: bool,
    pub accent: String,
    pub reduce_motion: bool,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            scheme: Scheme::System,
            design: Design::Fashion,
            system_accent: true,
            accent: "#6875e8".into(),
            reduce_motion: false,
        }
    }
}
impl Appearance {
    pub fn validate(&self) -> Result<(), String> {
        parse_color(&self.accent)
            .map(|_| ())
            .ok_or_else(|| "主题色需为 #RRGGBB".into())
    }
}
pub fn parse_color(s: &str) -> Option<u32> {
    (s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(&s[1..], 16).ok())
        .flatten()
}
fn luminance(rgb: u32) -> f64 {
    let f = |c: u32| {
        let v = c as f64 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f((rgb >> 16) & 255) + 0.7152 * f((rgb >> 8) & 255) + 0.0722 * f(rgb & 255)
}
pub fn ink(rgb: u32) -> u32 {
    if luminance(rgb) > 0.179 {
        0x101318
    } else {
        0xffffff
    }
}
pub fn text_accent(rgb: u32, dark: bool) -> u32 {
    let bg = if dark { 0x222735 } else { 0xf2f4fa };
    let base = luminance(bg);
    let mut out = rgb;
    for step in 0..=20 {
        let l = luminance(out);
        if (l.max(base) + 0.05) / (l.min(base) + 0.05) >= 4.5 {
            return out;
        }
        let t = (step + 1) as f64 / 20.;
        let mix = |c: u32| {
            if dark {
                (c as f64 + (255. - c as f64) * t).min(255.) as u32
            } else {
                (c as f64 * (1. - t)).max(0.) as u32
            }
        };
        out = (mix((rgb >> 16) & 255) << 16) | (mix((rgb >> 8) & 255) << 8) | mix(rgb & 255);
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_and_old_settings() {
        assert_eq!(parse_color("#aBc123"), Some(0xabc123));
        assert_eq!(parse_color("#１２３４５６"), None);
        assert_eq!(ink(0xffffff), 0x101318);
        assert_eq!(ink(0), 0xffffff);
        assert!(luminance(text_accent(0, true)) > 0.3);
        assert!(luminance(text_accent(0xffffff, false)) < 0.3);
        let s: crate::settings::Settings = serde_json::from_str("{\"version\":1}").unwrap();
        assert_eq!(s.appearance.scheme, Scheme::System);
        assert!(s.library.roots.is_empty());
    }
}
