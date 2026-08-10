//! Host-neutral Servokit value types shared by embedder and platform hosts.

use std::error::Error;
use std::fmt;

use raw_window_handle::{DisplayHandle, WindowHandle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostSurface {
    id: String,
}

impl HostSurface {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

impl SurfaceSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Physical-pixel origin of an embedded surface within its app-owned parent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePoint {
    pub x: i32,
    pub y: i32,
}

impl SurfacePoint {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }
}

/// Physical viewport for an embedded Servo surface.
///
/// `origin` is relative to the app-owned parent surface/layout; `size` is the renderable
/// slot in physical pixels; `scale_factor` is the display density that Servo must receive
/// before painting that size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceViewport {
    pub origin: SurfacePoint,
    pub size: SurfaceSize,
    pub scale_factor: f32,
}

impl SurfaceViewport {
    pub fn new(size: SurfaceSize, scale_factor: f32) -> Self {
        Self::with_origin(SurfacePoint::zero(), size, scale_factor)
    }

    pub fn with_origin(origin: SurfacePoint, size: SurfaceSize, scale_factor: f32) -> Self {
        Self {
            origin,
            size,
            scale_factor,
        }
    }

    pub fn with_size(self, size: SurfaceSize) -> Self {
        Self { size, ..self }
    }

    pub fn with_scale_factor(self, scale_factor: f32) -> Self {
        Self {
            scale_factor,
            ..self
        }
    }
}

/// Host-neutral borrowed native handles for an app-owned window, child surface, or parent surface.
#[derive(Debug, Clone, Copy)]
pub struct NativeChildSurface<'a> {
    display_handle: DisplayHandle<'a>,
    window_handle: WindowHandle<'a>,
}

impl<'a> NativeChildSurface<'a> {
    pub fn new(display_handle: DisplayHandle<'a>, window_handle: WindowHandle<'a>) -> Self {
        Self {
            display_handle,
            window_handle,
        }
    }

    pub fn display_handle(&self) -> DisplayHandle<'a> {
        self.display_handle
    }

    pub fn window_handle(&self) -> WindowHandle<'a> {
        self.window_handle
    }
}

/// The host-facing rendering mode currently backing a surface attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceMode {
    NativeChild,
    CpuOffscreen,
    GpuLayer,
}

/// Host-neutral description of a CPU-readback offscreen Servo target composited by the host.
#[derive(Debug, Clone, Copy)]
pub struct CpuOffscreenSurface<'a> {
    parent_surface: NativeChildSurface<'a>,
    parent_size: SurfaceSize,
}

impl<'a> CpuOffscreenSurface<'a> {
    pub fn new(parent_surface: NativeChildSurface<'a>, parent_size: SurfaceSize) -> Self {
        Self {
            parent_surface,
            parent_size,
        }
    }

    pub fn parent_surface(&self) -> NativeChildSurface<'a> {
        self.parent_surface
    }

    pub fn parent_size(&self) -> SurfaceSize {
        self.parent_size
    }
}

/// Host-neutral render target vocabulary shared by facade adapters and hosts.
#[derive(Debug, Clone, Copy)]
pub enum SurfaceTarget<'a> {
    NativeChild(NativeChildSurface<'a>),
    CpuOffscreen(CpuOffscreenSurface<'a>),
}

impl<'a> SurfaceTarget<'a> {
    pub fn native_child(native_surface: NativeChildSurface<'a>) -> Self {
        Self::NativeChild(native_surface)
    }

    pub fn cpu_offscreen(offscreen_surface: CpuOffscreenSurface<'a>) -> Self {
        Self::CpuOffscreen(offscreen_surface)
    }

    pub fn mode(&self) -> SurfaceMode {
        match self {
            Self::NativeChild(_) => SurfaceMode::NativeChild,
            Self::CpuOffscreen(_) => SurfaceMode::CpuOffscreen,
        }
    }
}

impl<'a> From<NativeChildSurface<'a>> for SurfaceTarget<'a> {
    fn from(native_surface: NativeChildSurface<'a>) -> Self {
        Self::native_child(native_surface)
    }
}

impl<'a> From<CpuOffscreenSurface<'a>> for SurfaceTarget<'a> {
    fn from(offscreen_surface: CpuOffscreenSurface<'a>) -> Self {
        Self::cpu_offscreen(offscreen_surface)
    }
}

/// Host-neutral metadata for a frame produced for an attached surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceFrameInfo {
    viewport: SurfaceViewport,
    mode: SurfaceMode,
}

impl SurfaceFrameInfo {
    pub fn new(viewport: SurfaceViewport, mode: SurfaceMode) -> Self {
        Self { viewport, mode }
    }

    pub fn viewport(&self) -> SurfaceViewport {
        self.viewport
    }

    pub fn mode(&self) -> SurfaceMode {
        self.mode
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceError {
    message: String,
}

impl SurfaceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SurfaceError {}

/// Minimal host-neutral behavior required by `SurfaceDelegate`'s default frame presenter.
pub trait SurfaceFrameLike {
    fn present(&mut self);
}

/// Host-neutral surface callbacks shared across facade adapters.
///
/// Higher layers may add adapter-specific identity such as `WebViewHandle`, but this trait stays
/// focused on values owned by `servokit-host`.
pub trait SurfaceDelegate {
    type Frame<'a>: SurfaceFrameLike;

    fn render_target(
        &mut self,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError>;

    fn update_render_target(
        &mut self,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError> {
        self.render_target(surface, viewport)
    }

    fn before_update(
        &mut self,
        _surface: Option<&HostSurface>,
        _viewport: Option<SurfaceViewport>,
    ) -> Result<(), SurfaceError> {
        Ok(())
    }

    fn present_frame(
        &mut self,
        _surface: &HostSurface,
        frame: &mut Self::Frame<'_>,
    ) -> Result<(), SurfaceError> {
        frame.present();
        Ok(())
    }
}
