//! Bounded LRC parsing, millisecond timestamps and stable duplicate timestamps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LyricLine {
    pub milliseconds: i64,
    pub text: String,
}
#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub plain: String,
}
impl Lyrics {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 1_000_000 {
            return Err("歌词文件超过 1MB".into());
        }
        let mut result = Self::default();
        let mut offset = 0i64;
        for raw in text.trim_start_matches('\u{feff}').lines() {
            let mut line = raw.trim();
            let mut times = Vec::new();
            while let Some(rest) = line.strip_prefix('[') {
                let Some((tag, tail)) = rest.split_once(']') else {
                    break;
                };
                line = tail;
                if let Some(value) = tag.strip_prefix("offset:") {
                    offset = value.trim().parse().map_err(|_| "非法 LRC offset")?;
                    if !(-600_000..=600_000).contains(&offset) {
                        return Err("LRC offset 超出范围".into());
                    }
                } else if let Some((minutes, seconds)) = tag.split_once(':')
                    && let (Ok(m), Ok(s)) = (minutes.parse::<i64>(), seconds.parse::<f64>())
                {
                    if !(0..=10000).contains(&m) || !s.is_finite() || !(0.0..60.0).contains(&s) {
                        return Err("非法 LRC 时间戳".into());
                    }
                    times.push(m * 60_000 + (s * 1000.0).round() as i64);
                }
            }
            if !times.is_empty() {
                for milliseconds in times {
                    result.lines.push(LyricLine {
                        milliseconds,
                        text: line.trim().into(),
                    });
                }
            } else if !line.trim().is_empty() {
                result.plain.push_str(line);
                result.plain.push('\n');
            }
            if result.lines.len() > 10000 {
                return Err("歌词行数超过 10000".into());
            }
        }
        for line in &mut result.lines {
            line.milliseconds = (line.milliseconds + offset).max(0);
        }
        result.lines.sort_by_key(|line| line.milliseconds);
        Ok(result)
    }
    pub fn active(&self, position_ms: i64, user_offset_ms: i64) -> Option<usize> {
        self.lines
            .partition_point(|line| line.milliseconds <= position_ms.saturating_add(user_offset_ms))
            .checked_sub(1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_offset_seek_and_bounds() {
        let lyrics =
            Lyrics::parse("[offset:-100]\n[00:02.50][00:01.20]Second\n[00:01.20]Translation")
                .unwrap();
        assert_eq!(lyrics.lines[0].milliseconds, 1100);
        assert_eq!(lyrics.active(1100, 0), Some(1));
        assert_eq!(lyrics.active(100, 0), None);
        assert!(Lyrics::parse("[00:99]bad").is_err());
        assert!(Lyrics::parse("[offset:99999999999]").is_err());
    }
}
