//! Thin ABI for the headers in the pinned Windows archive (client API 2).
//! No ordinary handle calls are allowed from the renderer thread.
use libloading::Library;
use serde_json::{Map, Value};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::Path;

pub type Handle = *mut c_void;
#[repr(C)]
pub struct Event {
    pub id: c_int,
    pub error: c_int,
    pub userdata: u64,
    pub data: *mut c_void,
}
#[repr(C)]
pub struct EndFile {
    pub reason: c_int,
    pub error: c_int,
    pub playlist_id: i64,
    pub insert_id: i64,
    pub insert_count: c_int,
}
#[repr(C)]
pub union NodeData {
    pub string: *mut c_char,
    pub flag: c_int,
    pub integer: i64,
    pub double: f64,
    pub list: *mut NodeList,
}
#[repr(C)]
pub struct Node {
    pub data: NodeData,
    pub format: c_int,
}
#[repr(C)]
pub struct NodeList {
    pub count: c_int,
    pub values: *mut Node,
    pub keys: *mut *mut c_char,
}
#[repr(C)]
pub struct RenderParam {
    pub kind: c_int,
    pub data: *mut c_void,
}
impl RenderParam {
    pub fn end() -> Self {
        Self {
            kind: 0,
            data: std::ptr::null_mut(),
        }
    }
}
#[repr(C)]
pub struct GlInit {
    pub get_proc: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    pub context: *mut c_void,
}
#[repr(C)]
pub struct GlFbo {
    pub fbo: c_int,
    pub width: c_int,
    pub height: c_int,
    pub format: c_int,
}
#[repr(C)]
pub struct FrameInfo {
    pub flags: u64,
    pub target_time: i64,
}
pub type Callback = Option<unsafe extern "C" fn(*mut c_void)>;

macro_rules! api {
    ($($field:ident: $ty:ty = $name:literal),* $(,)?) => {
        pub struct Api { _library: Library, $(pub $field: $ty),* }
        impl Api {
            pub fn load(path: &Path) -> Result<Self, String> {
                // SAFETY: the caller verifies an absolute controlled DLL and its hash.
                // Keep the library alive for all copied function pointers.
                let library: Library = unsafe {
                    #[cfg(windows)] { libloading::os::windows::Library::load_with_flags(path, 0x100 | 0x1000).map(Into::into) }
                    #[cfg(not(windows))] { Library::new(path) }
                }.map_err(|e| format!("无法加载 libmpv：{e}"))?;
                // SAFETY: function signatures match the pinned C headers; missing symbols fail initialization.
                unsafe { Ok(Self { $($field: *library.get::<$ty>(concat!($name, "\0").as_bytes()).map_err(|e| e.to_string())?),*, _library: library }) }
            }
            pub fn error(&self, code: i32) -> String {
                // SAFETY: mpv_error_string returns a static NUL-terminated message.
                unsafe { CStr::from_ptr((self.error_string)(code)).to_string_lossy().into_owned() }
            }
        }
    }
}
api! {
    version: unsafe extern "C" fn() -> std::ffi::c_ulong = "mpv_client_api_version",
    create: unsafe extern "C" fn() -> Handle = "mpv_create",
    initialize: unsafe extern "C" fn(Handle) -> c_int = "mpv_initialize",
    destroy: unsafe extern "C" fn(Handle) = "mpv_terminate_destroy",
    option: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int = "mpv_set_option_string",
    set: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int = "mpv_set_property_string",
    get_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char = "mpv_get_property_string",
    get: unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int = "mpv_get_property",
    free: unsafe extern "C" fn(*mut c_void) = "mpv_free",
    free_node: unsafe extern "C" fn(*mut Node) = "mpv_free_node_contents",
    command: unsafe extern "C" fn(Handle, u64, *const *const c_char) -> c_int = "mpv_command_async",
    wait: unsafe extern "C" fn(Handle, f64) -> *mut Event = "mpv_wait_event",
    observe: unsafe extern "C" fn(Handle, u64, *const c_char, c_int) -> c_int = "mpv_observe_property",
    wakeup: unsafe extern "C" fn(Handle, Callback, *mut c_void) = "mpv_set_wakeup_callback",
    error_string: unsafe extern "C" fn(c_int) -> *const c_char = "mpv_error_string",
    render_create: unsafe extern "C" fn(*mut Handle, Handle, *mut RenderParam) -> c_int = "mpv_render_context_create",
    render_free: unsafe extern "C" fn(Handle) = "mpv_render_context_free",
    render_callback: unsafe extern "C" fn(Handle, Callback, *mut c_void) = "mpv_render_context_set_update_callback",
    render_update: unsafe extern "C" fn(Handle) -> u64 = "mpv_render_context_update",
    render_info: unsafe extern "C" fn(Handle, RenderParam) -> c_int = "mpv_render_context_get_info",
    time_us: unsafe extern "C" fn(Handle) -> i64 = "mpv_get_time_us",
    time_ns: unsafe extern "C" fn(Handle) -> i64 = "mpv_get_time_ns",
    render: unsafe extern "C" fn(Handle, *mut RenderParam) -> c_int = "mpv_render_context_render",
}

/// # Safety
/// Only traverse successful mpv-owned nodes on the worker, before free_node
/// or wait_event invalidates them. Bound recursion and entry count defensively.
pub unsafe fn node_value(node: &Node, depth: usize) -> Value {
    if depth > 16 {
        return Value::Null;
    }
    unsafe {
        match node.format {
            1 if !node.data.string.is_null() => Value::String(
                CStr::from_ptr(node.data.string)
                    .to_string_lossy()
                    .into_owned(),
            ),
            3 => Value::Bool(node.data.flag != 0),
            4 => Value::from(node.data.integer),
            5 => Value::from(node.data.double),
            7 | 8 if !node.data.list.is_null() => {
                let list = &*node.data.list;
                if list.count <= 0 || list.count > 100_000 || list.values.is_null() {
                    return if node.format == 7 {
                        Value::Array(vec![])
                    } else {
                        Value::Object(Map::new())
                    };
                }
                let values = std::slice::from_raw_parts(list.values, list.count as usize);
                if node.format == 7 {
                    Value::Array(
                        values
                            .iter()
                            .map(|value| node_value(value, depth + 1))
                            .collect(),
                    )
                } else {
                    if list.keys.is_null() {
                        return Value::Null;
                    }
                    let keys = std::slice::from_raw_parts(list.keys, list.count as usize);
                    Value::Object(
                        keys.iter()
                            .zip(values)
                            .filter(|(key, _)| !key.is_null())
                            .map(|(key, value)| {
                                (
                                    CStr::from_ptr(*key).to_string_lossy().into_owned(),
                                    node_value(value, depth + 1),
                                )
                            })
                            .collect(),
                    )
                }
            }
            _ => Value::Null,
        }
    }
}
