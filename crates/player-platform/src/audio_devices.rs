//! Stable ALSA hardware identities, enumerated only on the Engine worker.
pub fn direct_devices() -> Vec<(String, String)> {
    #[cfg(target_os = "linux")]
    {
        let Ok(pcm) = std::fs::read_to_string("/proc/asound/pcm") else {
            return vec![];
        };
        pcm.lines()
            .filter_map(|line| {
                if !line.contains("playback") {
                    return None;
                }
                let (numbers, description) = line.split_once(':')?;
                let (card, device) = numbers.split_once('-')?;
                let card: u32 = card.parse().ok()?;
                let device: u32 = device.parse().ok()?;
                let id = std::fs::read_to_string(format!("/proc/asound/card{card}/id")).ok()?;
                let id = id.trim();
                if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                    return None;
                }
                Some((
                    format!("alsa/hw:CARD={id},DEV={device}"),
                    format!(
                        "ALSA 直连硬件 · {id} · {}",
                        description.split(':').next()?.trim()
                    ),
                ))
            })
            .take(128)
            .collect()
    }
    #[cfg(not(target_os = "linux"))]
    vec![]
}
