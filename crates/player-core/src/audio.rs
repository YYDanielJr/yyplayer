//! Validated, UI-independent audio policy and parametric EQ presets.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputMode {
    #[default]
    Shared,
    PreferExclusive,
    StrictExclusive,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BandKind {
    #[default]
    Peak,
    LowShelf,
    HighShelf,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EqBand {
    pub frequency: f64,
    pub gain: f64,
    pub q: f64,
    #[serde(default)]
    pub kind: BandKind,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct EqPreset {
    pub name: String,
    pub enabled: bool,
    pub preamp: f64,
    pub auto_headroom: bool,
    pub bands: Vec<EqBand>,
}
impl Default for EqPreset {
    fn default() -> Self {
        Self {
            name: "平直 · 不处理".into(),
            enabled: false,
            preamp: 0.0,
            auto_headroom: true,
            bands: [
                31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
            ]
            .into_iter()
            .map(|frequency| EqBand {
                frequency,
                gain: 0.0,
                q: 1.414,
                kind: BandKind::Peak,
            })
            .collect(),
        }
    }
}
impl EqPreset {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 160
            || self.bands.len() > 32
            || !self.preamp.is_finite()
            || !(-36.0..=12.0).contains(&self.preamp)
        {
            return Err("预设名称、前置增益或频段数量不合法".into());
        }
        if self.bands.iter().any(|b| {
            !b.frequency.is_finite()
                || !(10.0..=24000.0).contains(&b.frequency)
                || !b.gain.is_finite()
                || !(-24.0..=24.0).contains(&b.gain)
                || !b.q.is_finite()
                || !(0.1..=20.0).contains(&b.q)
        }) {
            return Err("频率需为 10–24000Hz，增益 ±24dB，Q 为 0.1–20".into());
        }
        Ok(())
    }
    /// Conservative summed positive gain allowance. Not a promise against intersample peaks.
    pub fn applied_preamp(&self) -> f64 {
        if self.auto_headroom {
            self.preamp
                .min(-self.bands.iter().map(|b| b.gain.max(0.0)).sum::<f64>())
        } else {
            self.preamp
        }
    }
    pub fn filter(&self) -> Result<String, String> {
        self.validate()?;
        if !self.enabled {
            return Ok(String::new());
        }
        let mut filters = Vec::new();
        let preamp = self.applied_preamp();
        if preamp.abs() > 0.001 {
            filters.push(format!("volume={preamp:.6}dB:precision=double"));
        }
        for band in self.bands.iter().filter(|b| b.gain.abs() > 0.001) {
            let filter = match band.kind {
                BandKind::Peak => "equalizer",
                BandKind::LowShelf => "bass",
                BandKind::HighShelf => "treble",
            };
            filters.push(format!(
                "{filter}=f={:.6}:t=q:w={:.6}:g={:.6}:r=f64",
                band.frequency, band.q, band.gain
            ));
        }
        Ok(if filters.is_empty() {
            String::new()
        } else {
            format!("@yy-eq:lavfi=[{}]", filters.join(","))
        })
    }
    pub fn import(text: &str, fallback_name: &str) -> Result<Self, String> {
        if text.len() > 256_000 {
            return Err("EQ 预设超过 256KB".into());
        }
        if text.trim_start().starts_with('{') {
            let value: serde_json::Value =
                serde_json::from_str(text).map_err(|e| format!("预设 JSON：{e}"))?;
            if value.get("name").is_none() || value.get("bands").is_none() {
                return Err("EQ JSON 需要 name 与 bands 字段".into());
            }
            let preset: Self =
                serde_json::from_value(value).map_err(|e| format!("预设 JSON：{e}"))?;
            preset.validate()?;
            return Ok(preset);
        }
        let mut preset = Self {
            name: fallback_name.into(),
            enabled: true,
            bands: Vec::new(),
            ..Self::default()
        };
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<_> = line.split_whitespace().collect();
            let number_after = |key: &str| -> Result<f64, String> {
                parts
                    .iter()
                    .position(|p| p.eq_ignore_ascii_case(key))
                    .and_then(|i| parts.get(i + 1))
                    .ok_or_else(|| format!("缺少 {key}"))?
                    .parse()
                    .map_err(|_| format!("非法 {key}"))
            };
            if parts[0].eq_ignore_ascii_case("Preamp:") {
                preset.preamp = parts
                    .get(1)
                    .ok_or("缺少前置增益")?
                    .parse()
                    .map_err(|_| "非法前置增益")?;
            } else if parts[0]
                .trim_end_matches(':')
                .eq_ignore_ascii_case("Filter")
            {
                if parts.iter().any(|p| p.eq_ignore_ascii_case("OFF")) {
                    continue;
                }
                if !parts.iter().any(|p| p.eq_ignore_ascii_case("ON")) {
                    return Err("Filter 缺少 ON / OFF".into());
                }
                let kind = if parts.iter().any(|p| p.eq_ignore_ascii_case("PK")) {
                    BandKind::Peak
                } else if parts.iter().any(|p| p.eq_ignore_ascii_case("LS")) {
                    BandKind::LowShelf
                } else if parts.iter().any(|p| p.eq_ignore_ascii_case("HS")) {
                    BandKind::HighShelf
                } else {
                    return Err("仅支持 PK / LS / HS 参数滤波器".into());
                };
                preset.bands.push(EqBand {
                    frequency: number_after("Fc")?,
                    gain: number_after("Gain")?,
                    q: number_after("Q")?,
                    kind,
                });
            } else if parts[0].eq_ignore_ascii_case("GraphicEQ:") {
                return Err(
                    "GraphicEQ 曲线不等价于参数 EQ，请导出带 Q 的 PK / LS / HS 预设".into(),
                );
            } else {
                return Err(format!("不支持的预设行：{line}"));
            }
        }
        if preset.bands.is_empty() {
            return Err("预设没有有效频段".into());
        }
        preset.validate()?;
        Ok(preset)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub mode: OutputMode,
    pub preserve_rate: bool,
    pub global_eq: EqPreset,
    pub presets: BTreeMap<String, EqPreset>,
    pub device_presets: BTreeMap<String, String>,
    pub file_eq: BTreeMap<PathBuf, EqPreset>,
    pub lyric_files: BTreeMap<PathBuf, PathBuf>,
    pub lyric_offset_ms: i64,
}
impl AudioSettings {
    pub fn validate(&self) -> Result<(), String> {
        self.global_eq.validate()?;
        if self.presets.len() > 256
            || self.file_eq.len() > 2000
            || self.lyric_files.len() > 2000
            || self.device_presets.len() > 256
            || !(-600_000..=600_000).contains(&self.lyric_offset_ms)
        {
            return Err("音频规则或歌词偏移超出限制".into());
        }
        for preset in self.presets.values().chain(self.file_eq.values()) {
            preset.validate()?;
        }
        if self
            .device_presets
            .values()
            .any(|name| !self.presets.contains_key(name))
        {
            return Err("设备绑定的 EQ 预设不存在".into());
        }
        Ok(())
    }
    pub fn resolve(&self, path: Option<&Path>, device: &str) -> (EqPreset, String) {
        if let Some(eq) = path.and_then(|p| self.file_eq.get(p)) {
            return (eq.clone(), "单文件".into());
        }
        if let Some(eq) = self
            .device_presets
            .get(device)
            .and_then(|name| self.presets.get(name))
        {
            return (eq.clone(), "设备绑定".into());
        }
        (self.global_eq.clone(), "全局".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_and_guard_graph() {
        let eq = EqPreset::import(
            "Preamp: -3 dB\nFilter 1: ON PK Fc 100 Hz Gain 6 dB Q 1.4",
            "test",
        )
        .unwrap();
        assert_eq!(eq.applied_preamp(), -6.0);
        assert!(eq.filter().unwrap().starts_with("@yy-eq:lavfi=[volume=-6"));
        assert!(EqPreset::import("Filter 1: ON PK Fc NaN Hz Gain 2 dB Q 1.4", "bad").is_err());
        assert!(EqPreset::import("Include: remote.txt", "bad").is_err());
        assert_eq!(
            EqPreset {
                enabled: true,
                ..Default::default()
            }
            .filter()
            .unwrap(),
            ""
        );
    }
    #[test]
    fn file_device_global_precedence() {
        let mut settings = AudioSettings::default();
        let preset = EqPreset {
            name: "DAC".into(),
            ..Default::default()
        };
        settings.presets.insert("DAC".into(), preset.clone());
        settings
            .device_presets
            .insert("wasapi/id".into(), "DAC".into());
        assert_eq!(
            settings
                .resolve(Some(Path::new("song.flac")), "wasapi/id")
                .1,
            "设备绑定"
        );
        settings
            .file_eq
            .insert("song.flac".into(), EqPreset::default());
        assert_eq!(
            settings
                .resolve(Some(Path::new("song.flac")), "wasapi/id")
                .1,
            "单文件"
        );
        assert_eq!(
            settings.resolve(Some(Path::new("else.flac")), "auto").1,
            "全局"
        );
    }
    #[test]
    fn old_settings_defaults_and_extreme_lyric_offset_are_safe() {
        let old: crate::settings::Settings =
            serde_json::from_str(r#"{"version":1,"volume":70,"device":"auto"}"#).unwrap();
        assert_eq!(old.audio.mode, OutputMode::Shared);
        assert_eq!(old.audio.global_eq.filter().unwrap(), "");
        let invalid = AudioSettings {
            lyric_offset_ms: i64::MIN,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
        assert!(EqPreset::import("GraphicEQ: 100 2; 1000 -3", "approximation").is_err());
    }
}
