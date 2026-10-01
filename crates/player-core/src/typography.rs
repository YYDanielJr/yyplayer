use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Fonts {
    pub ui: String,
    pub lyrics: String,
    pub subtitles: String,
    pub override_ass: bool,
}
impl Fonts {
    pub fn valid_family(name: &str) -> bool {
        name.chars().count() <= 128
            && !name
                .chars()
                .any(|c| c.is_control() || matches!(c, ',' | '=' | '\\'))
    }
    pub fn validate(&self) -> Result<(), String> {
        if [&self.ui, &self.lyrics, &self.subtitles]
            .into_iter()
            .all(|f| Self::valid_family(f))
        {
            Ok(())
        } else {
            Err("字体名称过长或含不支持的字符".into())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryColumns {
    pub song: f32,
    pub artist: f32,
}
impl Default for LibraryColumns {
    fn default() -> Self {
        Self {
            song: 0.46,
            artist: 0.24,
        }
    }
}
impl LibraryColumns {
    pub fn validate(&self) -> Result<(), String> {
        if self.song.is_finite()
            && self.artist.is_finite()
            && (0.2..=0.7).contains(&self.song)
            && (0.1..=0.5).contains(&self.artist)
            && self.song + self.artist <= 0.85
        {
            Ok(())
        } else {
            Err("音乐库列宽比例无效".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_config_and_unsafe_font_or_column_data() {
        let settings: crate::settings::Settings = serde_json::from_str("{\"version\":1}").unwrap();
        assert_eq!(settings.fonts, Fonts::default());
        for family in ["Arial,Default.Bold=1", "a\n", "x=y", "a\\b"] {
            assert!(!Fonts::valid_family(family));
        }
        assert!(Fonts::valid_family("思源黑体 CN"));
        assert!(LibraryColumns::default().validate().is_ok());
        assert!(
            LibraryColumns {
                song: f32::NAN,
                artist: 0.2
            }
            .validate()
            .is_err()
        );
        assert!(
            LibraryColumns {
                song: 0.7,
                artist: 0.5
            }
            .validate()
            .is_err()
        );
    }
}
