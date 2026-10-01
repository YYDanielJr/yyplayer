//! Installed font families, enumerated off the UI thread by the application.
use std::collections::BTreeSet;
pub fn enumerate() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    let families = windows_families()?;
    #[cfg(not(windows))]
    let families = {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        db.faces()
            .flat_map(|f| f.families.iter().map(|(name, _)| name.clone()))
            .collect::<BTreeSet<_>>()
    };
    let mut result: Vec<_> = families
        .into_iter()
        .filter(|f| !f.is_empty() && !f.starts_with('@'))
        .take(4096)
        .collect();
    result.sort_by_cached_key(|f| f.to_lowercase());
    if result.is_empty() {
        return Err("没有读到系统字体，请稍后刷新".into());
    }
    Ok(result)
}
#[cfg(windows)]
fn windows_families() -> Result<BTreeSet<String>, String> {
    use windows_sys::Win32::Graphics::Gdi::*;
    unsafe extern "system" fn collect(
        font: *const LOGFONTW,
        _: *const TEXTMETRICW,
        _: u32,
        context: isize,
    ) -> i32 {
        // SAFETY: EnumFontFamiliesExW calls synchronously with a valid LOGFONTW;
        // context points to the exclusive set owned by this enumeration invocation.
        let (font, families) = unsafe { (&*font, &mut *(context as *mut BTreeSet<String>)) };
        let end = font
            .lfFaceName
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(font.lfFaceName.len());
        if let Ok(name) = String::from_utf16(&font.lfFaceName[..end]) {
            families.insert(name);
        }
        i32::from(families.len() < 4096)
    }
    let mut families = BTreeSet::new();
    // SAFETY: null requests the desktop DC. It is used and released on this worker
    // thread; zeroed LOGFONTW requests all families with DEFAULT_CHARSET.
    unsafe {
        let dc = GetDC(std::ptr::null_mut());
        if dc.is_null() {
            return Err("无法读取系统字体（桌面 DC 不可用）".into());
        }
        let mut filter: LOGFONTW = std::mem::zeroed();
        filter.lfCharSet = DEFAULT_CHARSET;
        EnumFontFamiliesExW(
            dc,
            &filter,
            Some(collect),
            &mut families as *mut BTreeSet<String> as isize,
            0,
        );
        ReleaseDC(std::ptr::null_mut(), dc);
    }
    Ok(families)
}
