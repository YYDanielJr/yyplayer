//! Optional DWM rounded corners; a borrowed handle never outlives its host window.
#[derive(Clone, Debug, Default)]
pub struct SurfaceState {
    pub preference: Option<i32>,
    pub result: Option<i32>,
}
pub fn apply(handle: raw_window_handle::WindowHandle<'_>, rounded: bool) -> SurfaceState {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Graphics::Dwm::*;
        if let raw_window_handle::RawWindowHandle::Win32(raw) = handle.as_raw() {
            let hwnd = raw.hwnd.get() as windows_sys::Win32::Foundation::HWND;
            let preference = if rounded {
                DWMWCP_ROUND
            } else {
                DWMWCP_DONOTROUND
            };
            let mut actual = 0i32;
            // SAFETY: handle is borrowed from the live Winit window on its owning
            // UI thread. i32 matches DWM_WINDOW_CORNER_PREFERENCE and both buffers
            // remain valid throughout the synchronous DWM calls.
            let (result, read) = unsafe {
                let result = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    &preference as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
                );
                let read = DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    &mut actual as *mut _ as *mut _,
                    std::mem::size_of::<i32>() as u32,
                );
                (result, read)
            };
            return SurfaceState {
                result: Some(result),
                preference: (read >= 0).then_some(actual),
            };
        }
    }
    #[cfg(not(windows))]
    let _ = (handle, rounded);
    SurfaceState::default()
}
