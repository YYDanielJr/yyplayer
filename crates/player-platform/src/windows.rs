use crate::{PlatformInfo, PlatformKind};

pub(crate) fn info() -> PlatformInfo {
    PlatformInfo {
        kind: PlatformKind::Windows,
        name: "Windows",
    }
}
