use crate::{PlatformInfo, PlatformKind};

pub(crate) fn info() -> PlatformInfo {
    PlatformInfo {
        kind: PlatformKind::Linux,
        name: "Linux · Wayland / X11",
    }
}
