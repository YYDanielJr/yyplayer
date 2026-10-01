//! Compare static fixture GPU screenshots. State/exit-code tests alone cannot
//! detect visible corruption. This helper never participates in playback.
use image::GenericImageView;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let reference = arguments.next().ok_or("reference PNG required")?;
    let candidate = arguments.next().ok_or("candidate PNG required")?;
    let report = arguments.next();
    let reference = image::open(reference)?.to_rgb8();
    let candidate = image::open(candidate)?.to_rgb8();
    if reference.dimensions() != candidate.dimensions() {
        return Err("Screenshot dimensions differ; use matching window and DPI".into());
    }
    let (width, height) = reference.dimensions();
    let region = (
        width * 3 / 100,
        height * 15 / 100,
        width * 94 / 100,
        height * 65 / 100,
    );
    // Interior of the static 16:9 fixture; exclude controls and letterboxing.
    let reference = reference.view(region.0, region.1, region.2, region.3);
    let candidate = candidate.view(region.0, region.1, region.2, region.3);
    let mut error = 0u64;
    let mut bad_pixels = 0u64;
    let total = u64::from(region.2) * u64::from(region.3);
    for (reference, candidate) in reference.pixels().zip(candidate.pixels()) {
        let differences: [u8; 3] =
            std::array::from_fn(|channel| reference.2[channel].abs_diff(candidate.2[channel]));
        error += differences
            .iter()
            .map(|value| u64::from(*value))
            .sum::<u64>();
        bad_pixels += u64::from(differences.iter().any(|value| *value > 12));
    }
    let mean_error = error as f64 / (total as f64 * 3.0);
    let bad_fraction = bad_pixels as f64 / total as f64;
    let passed = mean_error <= 1.5 && bad_fraction <= 0.01;
    let metrics = serde_json::json!({ "passed": passed, "mean_absolute_rgb_error": mean_error, "bad_pixel_fraction": bad_fraction, "channel_threshold": 12, "max_mean_error": 1.5, "max_bad_fraction": 0.01, "region": region, "width": width, "height": height });
    let data = serde_json::to_string_pretty(&metrics)?;
    println!("{data}");
    if let Some(path) = report {
        std::fs::write(path, data)?;
    }
    if !passed {
        return Err("GPU output differs excessively from software reference".into());
    }
    Ok(())
}
