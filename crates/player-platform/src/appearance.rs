//! Read-only OS personalization observer. No registry writes or desktop effects.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemAppearance {
    pub dark: Option<bool>,
    pub accent: Option<u32>,
    pub reduce_motion: bool,
}
pub struct Observer {
    state: Arc<Mutex<SystemAppearance>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Observer {
    pub fn start() -> Self {
        let state = Arc::new(Mutex::new(SystemAppearance::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let snapshot = state.clone();
        let stopped = stop.clone();
        let worker = std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let value = read();
                if let Ok(mut s) = snapshot.lock() {
                    *s = value;
                }
                for _ in 0..20 {
                    if stopped.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        });
        Self {
            state,
            stop,
            worker: Some(worker),
        }
    }
    pub fn snapshot(&self) -> SystemAppearance {
        self.state.lock().map(|s| *s).unwrap_or_default()
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
#[cfg(not(windows))]
fn read() -> SystemAppearance {
    SystemAppearance::default()
}
#[cfg(windows)]
fn read() -> SystemAppearance {
    use windows_sys::Win32::{
        Graphics::Dwm::DwmGetColorizationColor, System::Registry::*, UI::WindowsAndMessaging::*,
    };
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = "AppsUseLightTheme".encode_utf16().chain(Some(0)).collect();
    let mut light = 1u32;
    let mut size = 4u32;
    let mut accent = 0u32;
    let mut opaque = 0i32;
    let mut animation = 1i32;
    // SAFETY: all read-only Win32 calls receive valid live buffers of the declared size.
    let (ok, color, anim) = unsafe {
        (
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut light as *mut u32).cast(),
                &mut size,
            ),
            DwmGetColorizationColor(&mut accent, &mut opaque),
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                (&mut animation as *mut i32).cast(),
                0,
            ),
        )
    };
    SystemAppearance {
        dark: (ok == 0).then_some(light == 0),
        accent: (color >= 0).then_some(accent & 0xffffff),
        reduce_motion: anim != 0 && animation == 0,
    }
}
