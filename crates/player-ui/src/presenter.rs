//! GPU-only libmpv composition. All GL and mpv_render_* calls stay inside Slint's
//! notifier with its current context. No ordinary mpv calls or waits on worker.
use crate::AppWindow;
use glow::HasContext;
use player_mpv::{RenderBridge, ffi};
use slint::{ComponentHandle, GraphicsAPI, RenderingState};
use std::ffi::{CStr, c_char, c_void};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use std::{cell::Cell, rc::Rc};

struct Signal {
    pending: AtomicBool,
    notify: Box<dyn Fn() + Send + Sync>,
}
unsafe extern "C" fn update(context: *mut c_void) {
    // SAFETY: signal remains boxed until callback removed and render_free completed.
    let signal = unsafe { &*(context as *const Signal) };
    if !signal.pending.swap(true, Ordering::AcqRel) {
        (signal.notify)();
    }
}
struct Resolver<'a> {
    get: &'a dyn Fn(&CStr) -> *const c_void,
}
unsafe extern "C" fn resolve(context: *mut c_void, name: *const c_char) -> *mut c_void {
    // SAFETY: mpv only uses resolver synchronously during render_create (per render_gl.h).
    let resolver = unsafe { &*(context as *const Resolver<'_>) };
    (resolver.get)(unsafe { CStr::from_ptr(name) }) as *mut c_void
}
struct Renderer {
    gl: glow::Context,
    bridge: Arc<RenderBridge>,
    render: ffi::Handle,
    core_handle: ffi::Handle,
    signal: Box<Signal>,
    texture: glow::NativeTexture,
    fbo: glow::NativeFramebuffer,
    size: (i32, i32),
    retired: Vec<(glow::NativeTexture, glow::NativeFramebuffer)>,
    pending_frame: bool,
    scheduled: Rc<Cell<bool>>,
    #[cfg(debug_assertions)]
    captured: bool,
    #[cfg(debug_assertions)]
    trace_count: u32,
}
impl Renderer {
    fn new(
        get: &dyn Fn(&CStr) -> *const c_void,
        bridge: Arc<RenderBridge>,
        window: &AppWindow,
    ) -> Result<Self, String> {
        let handle = bridge.acquire().ok_or("播放内核正在退出或已被呈现器使用")?;
        let creation = || -> Result<Self, String> {
            // SAFETY: Slint guarantees current NativeOpenGL context throughout notifier.
            let gl = unsafe { glow::Context::from_loader_function_cstr(|name| get(name)) };
            prepare_gl(&gl);
            let mut resolver = Resolver { get };
            let mut init = ffi::GlInit {
                get_proc: resolve,
                context: (&mut resolver as *mut Resolver<'_>).cast(),
            };
            let mut advanced = 1i32;
            let mut parameters = [
                ffi::RenderParam {
                    kind: 1,
                    data: c"opengl".as_ptr() as *mut c_void,
                },
                ffi::RenderParam {
                    kind: 2,
                    data: (&mut init as *mut ffi::GlInit).cast(),
                },
                ffi::RenderParam {
                    kind: 10,
                    data: (&mut advanced as *mut i32).cast(),
                },
                ffi::RenderParam::end(),
            ];
            let mut render = std::ptr::null_mut();
            let result =
                unsafe { (bridge.api.render_create)(&mut render, handle, parameters.as_mut_ptr()) };
            if result < 0 {
                return Err(format!(
                    "OpenGL 视频初始化失败：{}",
                    bridge.api.error(result)
                ));
            }
            // Create resources before installing callbacks; cleanup render on allocation failure.
            let resources = unsafe {
                gl.create_texture()
                    .and_then(|texture| match gl.create_framebuffer() {
                        Ok(fbo) => Ok((texture, fbo)),
                        Err(e) => {
                            gl.delete_texture(texture);
                            Err(e)
                        }
                    })
            };
            let (texture, fbo) = match resources {
                Ok(resources) => resources,
                Err(e) => {
                    unsafe {
                        (bridge.api.render_free)(render);
                    }
                    return Err(e);
                }
            };
            let weak = window.as_weak();
            let signal = Box::new(Signal {
                pending: AtomicBool::new(true),
                notify: Box::new(move || {
                    let weak = weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak.upgrade() {
                            window.window().request_redraw();
                        }
                    });
                }),
            });
            let mut renderer = Self {
                gl,
                bridge: bridge.clone(),
                render,
                core_handle: handle,
                signal,
                texture,
                fbo,
                size: (0, 0),
                retired: vec![],
                pending_frame: true,
                scheduled: Rc::new(Cell::new(false)),
                #[cfg(debug_assertions)]
                captured: false,
                #[cfg(debug_assertions)]
                trace_count: 0,
            };
            unsafe {
                (bridge.api.render_callback)(
                    render,
                    Some(update),
                    (&mut *renderer.signal as *mut Signal).cast(),
                );
            }
            bridge.ready();
            Ok(renderer)
        };
        let result = creation();
        if result.is_err() {
            bridge.release();
        }
        result
    }
    fn draw(&mut self, window: &AppWindow) -> Result<(), String> {
        prepare_gl(&self.gl);
        self.signal.pending.store(false, Ordering::Release);
        // Safe render-only API, no locking or waiting on worker.
        let flags = unsafe { (self.bridge.api.render_update)(self.render) };
        self.pending_frame |= flags & 1 != 0;
        let scale = window.window().scale_factor();
        let width = (window.get_video_width() * scale)
            .round()
            .clamp(1.0, 7680.0) as i32;
        let height = (window.get_video_height() * scale)
            .round()
            .clamp(1.0, 4320.0) as i32;
        let mut info = ffi::FrameInfo {
            flags: 0,
            target_time: 0,
        };
        let result = unsafe {
            (self.bridge.api.render_info)(
                self.render,
                ffi::RenderParam {
                    kind: 11,
                    data: (&mut info as *mut ffi::FrameInfo).cast(),
                },
            )
        };
        if result < 0 {
            return Err(self.bridge.api.error(result));
        }
        // The pinned git build assigns vo_frame.pts (nanoseconds) directly to
        // target_time, despite render.h still saying microseconds. Compare both
        // documented clock bases, so patched/stable builds with us also work.
        let now_us = unsafe { (self.bridge.api.time_us)(self.core_handle) };
        let now_ns = unsafe { (self.bridge.api.time_ns)(self.core_handle) };
        let until = frame_delay_us(info.target_time, now_us, now_ns);
        #[cfg(debug_assertions)]
        if self.trace_count < 1000
            && let Some(path) = std::env::var_os("YYPLAYER_RENDER_TRACE")
        {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(
                    file,
                    "update={flags} frame={} until={until} pending={} scheduled={} size={:?}",
                    info.flags,
                    self.pending_frame,
                    self.scheduled.get(),
                    self.size
                );
            }
            self.trace_count += 1;
        }
        if !self.pending_frame && self.size == (width, height) {
            return Ok(());
        }
        if info.flags & 1 != 0 && until > 1500 {
            if !self.scheduled.replace(true) {
                let scheduled = self.scheduled.clone();
                let weak = window.as_weak();
                slint::Timer::single_shot(
                    Duration::from_micros(until.min(100_000) as u64),
                    move || {
                        scheduled.set(false);
                        if let Some(window) = weak.upgrade() {
                            window.window().request_redraw();
                        }
                    },
                );
            }
            return Ok(());
        }
        self.scheduled.set(false);
        // SAFETY: resources allocated by this exact context. Clear UI reference before
        // replacing texture; retire old resources after Slint's current frame renders.
        unsafe {
            if self.size != (width, height) {
                window.set_video_frame(slint::Image::default());
                if self.size != (0, 0) {
                    let texture = self.gl.create_texture()?;
                    let fbo = self.gl.create_framebuffer()?;
                    self.retired.push((self.texture, self.fbo));
                    self.texture = texture;
                    self.fbo = fbo;
                }
                self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    width,
                    height,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                self.gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                self.gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
                self.gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(self.texture),
                    0,
                );
                if self.gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE
                {
                    return Err("GPU 视频 framebuffer 不完整".into());
                }
                self.size = (width, height);
            }
            let mut fbo = ffi::GlFbo {
                fbo: self.fbo.0.get() as i32,
                width,
                height,
                format: glow::RGBA8 as i32,
            };
            // Slint samples borrowed textures with top-left origin; the FBO must
            // retain mpv's unflipped orientation (verified against source frame).
            let mut flip = 0i32;
            let mut block = 0i32;
            let mut params = [
                ffi::RenderParam {
                    kind: 3,
                    data: (&mut fbo as *mut ffi::GlFbo).cast(),
                },
                ffi::RenderParam {
                    kind: 4,
                    data: (&mut flip as *mut i32).cast(),
                },
                ffi::RenderParam {
                    kind: 12,
                    data: (&mut block as *mut i32).cast(),
                },
                ffi::RenderParam::end(),
            ];
            // Texture/FBO allocation on resize also changes GL state; restore
            // the contract again immediately before handing the context to mpv.
            prepare_gl(&self.gl);
            let result = (self.bridge.api.render)(self.render, params.as_mut_ptr());
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            if result < 0 {
                return Err(self.bridge.api.error(result));
            }
            let image = slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                self.texture.0,
                [width as u32, height as u32].into(),
            )
            .build();
            window.set_video_frame(image);
            window.set_render_ready(true);
            window.set_render_frames(window.get_render_frames() + 1);
            self.pending_frame = false;
        }
        Ok(())
    }
    fn after(&mut self) {
        // SAFETY: no live UI source references the retired images, current GL context.
        for (texture, fbo) in self.retired.drain(..) {
            unsafe {
                self.gl.delete_framebuffer(fbo);
                self.gl.delete_texture(texture);
            }
        }
    }
    #[cfg(debug_assertions)]
    fn capture(&mut self, window: &AppWindow) {
        if self.captured || window.get_render_frames() < 35 {
            return;
        }
        let Some(path) = std::env::var_os("YYPLAYER_UI_CAPTURE") else {
            return;
        };
        self.captured = true;
        let size = window.window().size();
        let width = size.width as usize;
        let height = size.height as usize;
        let mut pixels = vec![0u8; width * height * 4];
        // SAFETY: one explicit developer screenshot from the current default FBO,
        // after Slint rendered and before presentation. Never used in video output.
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
            self.gl.read_buffer(glow::BACK);
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
        }
        use std::io::Write;
        if let Ok(mut file) = std::fs::File::create(path) {
            let _ = write!(file, "P6\n{width} {height}\n255\n");
            for row in (0..height).rev() {
                let rgb: Vec<_> = pixels[row * width * 4..(row + 1) * width * 4]
                    .chunks_exact(4)
                    .flat_map(|pixel| pixel[..3].iter().copied())
                    .collect();
                let _ = file.write_all(&rgb);
            }
        }
    }
    fn destroy(mut self, window: Option<&AppWindow>) {
        prepare_gl(&self.gl);
        if let Some(window) = window {
            window.set_video_frame(slint::Image::default());
            window.set_render_ready(false);
        }
        // SAFETY: unregister and free while context current, signal and bridge outlive calls.
        unsafe {
            (self.bridge.api.render_callback)(self.render, None, std::ptr::null_mut());
            (self.bridge.api.render_free)(self.render);
            self.gl.delete_framebuffer(self.fbo);
            self.gl.delete_texture(self.texture);
        }
        self.after();
        self.bridge.release();
    }
}
/// libmpv's render_gl.h requires standard GL state on entry. FemtoVG leaves
/// blending and texture unit 1 active, so do not inherit the UI's state.
fn prepare_gl(gl: &glow::Context) {
    let modern = gl.version().major >= 3;
    let desktop = !gl.version().is_embedded;
    // SAFETY: called only inside Slint's notifier, with this context current.
    unsafe {
        for capability in [
            glow::BLEND,
            glow::SCISSOR_TEST,
            glow::DEPTH_TEST,
            glow::STENCIL_TEST,
            glow::CULL_FACE,
        ] {
            gl.disable(capability);
        }
        if modern {
            gl.disable(glow::RASTERIZER_DISCARD);
        }
        if modern && desktop {
            gl.disable(glow::FRAMEBUFFER_SRGB);
        }
        gl.color_mask(true, true, true, true);
        gl.depth_mask(true);
        gl.stencil_mask(u32::MAX);
        gl.blend_equation(glow::FUNC_ADD);
        gl.blend_func(glow::ONE, glow::ZERO);
        gl.use_program(None);
        if modern
            || gl
                .supported_extensions()
                .contains("GL_ARB_vertex_array_object")
        {
            gl.bind_vertex_array(None);
        }
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        if modern || desktop {
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
        }
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
        if modern || desktop {
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        }
    }
}
fn frame_delay_us(target: i64, now_us: i64, now_ns: i64) -> i64 {
    if target == 0 {
        return 0;
    }
    let micro_distance = (target as i128 - now_us as i128).abs();
    let nano_distance = (target as i128 - now_ns as i128).abs();
    if nano_distance < micro_distance {
        target.saturating_sub(now_ns) / 1000
    } else {
        target.saturating_sub(now_us)
    }
}
pub fn attach(
    window: &AppWindow,
    endpoint: Arc<Mutex<Option<Arc<RenderBridge>>>>,
) -> Result<(), slint::SetRenderingNotifierError> {
    let weak = window.as_weak();
    let mut renderer: Option<Renderer> = None;
    let mut failed = false;
    window.window().set_rendering_notifier(move |state, api| {
        let window = weak.upgrade();
        match state {
            RenderingState::BeforeRendering => {
                let Some(window) = window.as_ref() else {
                    return;
                };
                // Retain the lease until Engine observes the video output has closed.
                // Free only inside this notifier with the creating GL context current.
                if !window.get_video_renderer_requested() {
                    if let Some(renderer) = renderer.take() {
                        renderer.destroy(Some(window));
                    }
                    failed = false;
                    return;
                }
                if renderer.is_none() && !failed {
                    window.set_render_error("".into());
                    let bridge = endpoint.lock().unwrap().clone();
                    if let Some(bridge) = bridge {
                        if let GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                            match Renderer::new(get_proc_address, bridge, window) {
                                Ok(value) => {
                                    window.set_render_initializations(
                                        window.get_render_initializations() + 1,
                                    );
                                    renderer = Some(value);
                                }
                                Err(error) => {
                                    failed = true;
                                    window.set_render_error(error.into());
                                }
                            }
                        } else {
                            failed = true;
                            window.set_render_error(
                                "当前渲染器不是 NativeOpenGL，无法播放视频".into(),
                            );
                        }
                    }
                }
                if let Some(renderer) = renderer.as_mut()
                    && let Err(error) = renderer.draw(window)
                {
                    window.set_render_error(error.into());
                }
            }
            RenderingState::AfterRendering => {
                if let Some(renderer) = renderer.as_mut() {
                    renderer.after();
                    #[cfg(debug_assertions)]
                    if let Some(window) = window.as_ref() {
                        renderer.capture(window);
                    }
                }
            }
            RenderingState::RenderingTeardown => {
                if let Some(renderer) = renderer.take() {
                    renderer.destroy(window.as_ref());
                }
                failed = false;
            }
            _ => {}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::frame_delay_us;
    #[test]
    fn render_target_handles_pinned_ns_and_documented_us() {
        assert_eq!(
            frame_delay_us(2_033_000_000, 2_000_000, 2_000_000_000),
            33_000
        );
        assert_eq!(frame_delay_us(2_033_000, 2_000_000, 2_000_000_000), 33_000);
        assert_eq!(
            frame_delay_us(1_990_000_000, 2_000_000, 2_000_000_000),
            -10_000
        );
        assert_eq!(frame_delay_us(0, 2_000_000, 2_000_000_000), 0);
    }
}
