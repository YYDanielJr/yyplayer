use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutSettings {
    pub toggle_pause: String,
    pub seek_back: String,
    pub seek_forward: String,
    pub volume_up: String,
    pub volume_down: String,
    pub fullscreen: String,
    pub mute: String,
    pub info: String,
    pub open: String,
    pub frame_step: String,
    pub seek_seconds: f64,
    pub volume_step: f32,
    pub hold_ms: u64,
    pub hold_speed: f64,
}
impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            toggle_pause: "Space".into(),
            seek_back: "Left".into(),
            seek_forward: "Right".into(),
            volume_up: "Up".into(),
            volume_down: "Down".into(),
            fullscreen: "F".into(),
            mute: "M".into(),
            info: "Tab".into(),
            open: "Ctrl+O".into(),
            frame_step: "D".into(),
            seek_seconds: 5.0,
            volume_step: 5.0,
            hold_ms: 350,
            hold_speed: 3.0,
        }
    }
}
impl ShortcutSettings {
    pub fn bindings(&self) -> Vec<(&str, &str)> {
        vec![
            ("pause", &self.toggle_pause),
            ("back", &self.seek_back),
            ("forward", &self.seek_forward),
            ("up", &self.volume_up),
            ("down", &self.volume_down),
            ("fullscreen", &self.fullscreen),
            ("mute", &self.mute),
            ("info", &self.info),
            ("open", &self.open),
            ("frame", &self.frame_step),
        ]
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = BTreeSet::new();
        for (_, binding) in self.bindings() {
            let normalized = normalize(binding)?;
            if normalized == "Escape" || normalized == "Enter" {
                return Err("Escape / Enter 保留为退出 / 切换全屏".into());
            }
            if !seen.insert(normalized) {
                return Err("快捷键不能重复".into());
            }
        }
        if !self.seek_seconds.is_finite()
            || !(0.1..=600.0).contains(&self.seek_seconds)
            || !self.volume_step.is_finite()
            || !(1.0..=25.0).contains(&self.volume_step)
            || !(150..=2000).contains(&self.hold_ms)
            || !self.hold_speed.is_finite()
            || !(1.0..=8.0).contains(&self.hold_speed)
        {
            return Err("检查步长、长按阈值（150–2000ms）和临时速度（1–8）".into());
        }
        Ok(())
    }
    pub fn action(&self, chord: &str) -> Option<&str> {
        self.bindings()
            .into_iter()
            .find(|(_, value)| normalize(value).ok().as_deref() == Some(chord))
            .map(|(action, _)| action)
    }
}
pub fn normalize(binding: &str) -> Result<String, String> {
    let mut modifiers = BTreeSet::new();
    let parts: Vec<_> = binding.split('+').map(str::trim).collect();
    let key = parts.last().copied().unwrap_or("");
    for modifier in parts.iter().take(parts.len().saturating_sub(1)) {
        let modifier = match modifier.to_ascii_lowercase().as_str() {
            "ctrl" => "Ctrl",
            "alt" => "Alt",
            "shift" => "Shift",
            _ => return Err("修饰键只支持 Ctrl / Alt / Shift".into()),
        };
        if !modifiers.insert(modifier) {
            return Err("重复修饰键".into());
        }
    }
    let key = match key.to_ascii_lowercase().as_str() {
        "space" => "Space".into(),
        "left" => "Left".into(),
        "right" => "Right".into(),
        "up" => "Up".into(),
        "down" => "Down".into(),
        "tab" => "Tab".into(),
        "enter" => "Enter".into(),
        "escape" => "Escape".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Delete".into(),
        "insert" => "Insert".into(),
        "plus" => "Plus".into(),
        "minus" | "-" => "Minus".into(),
        k if k.starts_with('f')
            && k[1..]
                .parse::<u8>()
                .is_ok_and(|number| (1..=24).contains(&number)) =>
        {
            k.to_ascii_uppercase()
        }
        k if ["[", "]", ",", ".", ";", "/", "'", "="].contains(&k) => k.into(),
        k if k.len() == 1 && k.as_bytes()[0].is_ascii_alphanumeric() => k.to_ascii_uppercase(),
        _ => {
            return Err(
                "按键支持字母、数字、F1–F24、方向键、Home / End / PageUp / PageDown 等".into(),
            );
        }
    };
    let mut output = String::new();
    for modifier in ["Ctrl", "Alt", "Shift"] {
        if modifiers.contains(modifier) {
            output.push_str(modifier);
            output.push('+');
        }
    }
    output.push_str(&key);
    Ok(output)
}

#[derive(Clone, Debug)]
pub enum HoldEffect {
    Seek(f64),
    Speed(f64),
}
#[derive(Default)]
pub struct HoldGesture {
    held: Option<(String, u64, f64, f64)>,
    accelerated: bool,
}
impl HoldGesture {
    pub fn press(&mut self, key: String, now_ms: u64, seek: f64, speed: f64) {
        if self.held.is_none() {
            self.held = Some((key, now_ms, seek, speed));
            self.accelerated = false;
        }
    }
    pub fn tick(&mut self, now_ms: u64, threshold: u64, fast: f64) -> Option<HoldEffect> {
        if let Some((_, started, _, _)) = &self.held
            && !self.accelerated
            && now_ms.saturating_sub(*started) >= threshold
        {
            self.accelerated = true;
            return Some(HoldEffect::Speed(fast));
        }
        None
    }
    pub fn release(&mut self, key: &str) -> Option<HoldEffect> {
        if self
            .held
            .as_ref()
            .is_some_and(|(held, _, _, _)| held.rsplit('+').next() == key.rsplit('+').next())
        {
            let (_, _, seek, speed) = self.held.take()?;
            let accelerated = std::mem::take(&mut self.accelerated);
            return Some(if accelerated {
                HoldEffect::Speed(speed)
            } else {
                HoldEffect::Seek(seek)
            });
        }
        None
    }
    pub fn cancel(&mut self) -> Option<HoldEffect> {
        let (_, _, _, speed) = self.held.take()?;
        let accelerated = std::mem::take(&mut self.accelerated);
        accelerated.then_some(HoldEffect::Speed(speed))
    }
    pub fn active(&self) -> bool {
        self.accelerated
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tap_hold_repeat_and_focus_cancel() {
        let mut gesture = HoldGesture::default();
        gesture.press("Right".into(), 0, 5.0, 1.5);
        assert!(matches!(
            gesture.release("Right"),
            Some(HoldEffect::Seek(5.0))
        ));
        gesture.press("Right".into(), 100, 5.0, 1.5);
        gesture.press("Right".into(), 300, 5.0, 3.0);
        assert!(gesture.tick(449, 350, 3.0).is_none());
        assert!(matches!(
            gesture.tick(450, 350, 3.0),
            Some(HoldEffect::Speed(3.0))
        ));
        assert!(gesture.tick(600, 350, 3.0).is_none());
        assert!(matches!(gesture.cancel(), Some(HoldEffect::Speed(1.5))));
        assert!(gesture.release("Right").is_none());
        // Releasing the modifier before the main key must still restore speed.
        gesture.press("Ctrl+Left".into(), 1000, -5.0, 0.75);
        assert!(matches!(
            gesture.tick(1350, 350, 3.0),
            Some(HoldEffect::Speed(3.0))
        ));
        assert!(matches!(
            gesture.release("Left"),
            Some(HoldEffect::Speed(0.75))
        ));
        gesture.press("Left".into(), 1400, -5.0, 1.0);
        assert!(matches!(
            gesture.release("Left"),
            Some(HoldEffect::Seek(-5.0))
        ));
    }
    #[test]
    fn conflicts_and_invalid_ranges_are_rejected() {
        let mut config = ShortcutSettings::default();
        assert!(config.validate().is_ok());
        assert_eq!(normalize("shift+CTRL+right").unwrap(), "Ctrl+Shift+Right");
        assert!(normalize("Ctrl+Ctrl+Right").is_err());
        config.mute = "f".into();
        assert!(config.validate().is_err());
        config.mute = "M".into();
        config.hold_speed = f64::NAN;
        assert!(config.validate().is_err());
    }
}
