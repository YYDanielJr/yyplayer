use crate::{PlatformInfo, PlatformKind};

pub(crate) fn info() -> PlatformInfo {
    PlatformInfo {
        kind: PlatformKind::Linux,
        name: "Linux · Wayland / X11",
    }
}

/// Read desktop portal values on the appearance worker, with bounded method waits.
pub(crate) fn appearance() -> Option<crate::appearance::SystemAppearance> {
    use zbus::{
        blocking::{Proxy, connection::Builder},
        zvariant::OwnedValue,
    };
    let connection = Builder::session()
        .ok()?
        .method_timeout(std::time::Duration::from_millis(500))
        .build()
        .ok()?;
    let proxy = Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    )
    .ok()?;
    let read = |namespace: &str, key: &str| -> Option<OwnedValue> {
        // ReadOne has a single variant layer; older portals expose the legacy Read.
        proxy.call("ReadOne", &(namespace, key)).ok().or_else(|| {
            let value: OwnedValue = proxy.call("Read", &(namespace, key)).ok()?;
            value.downcast_ref::<OwnedValue>().ok().or(Some(value))
        })
    };
    let dark = read("org.freedesktop.appearance", "color-scheme").and_then(|v| {
        match v.downcast_ref::<u32>().ok()? {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    });
    let accent = read("org.freedesktop.appearance", "accent-color")
        .and_then(|v| v.downcast_ref::<(f64, f64, f64)>().ok())
        .and_then(accent_rgb);
    let reduce_motion = read("org.freedesktop.appearance", "reduced-motion")
        .and_then(|v| v.downcast_ref::<u32>().ok())
        .map(|v| v == 1)
        .or_else(|| {
            read("org.gnome.desktop.interface", "enable-animations")
                .and_then(|v| v.downcast_ref::<bool>().ok())
                .map(|v| !v)
        })
        .unwrap_or(false);
    Some(crate::appearance::SystemAppearance {
        dark,
        accent,
        reduce_motion,
    })
}
fn accent_rgb((r, g, b): (f64, f64, f64)) -> Option<u32> {
    if ![r, g, b]
        .iter()
        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    {
        return None;
    }
    Some(
        ((r * 255.0).round() as u32) << 16
            | ((g * 255.0).round() as u32) << 8
            | (b * 255.0).round() as u32,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_portal_color_is_unknown() {
        assert_eq!(accent_rgb((1.0, 0.5, 0.0)), Some(0xff8000));
        assert_eq!(accent_rgb((f64::NAN, 0.0, 0.0)), None);
        assert_eq!(accent_rgb((0.0, 1.01, 0.0)), None);
    }
}
