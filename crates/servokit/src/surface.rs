//! Surface facade exports.

pub use servokit_host::{
    CpuOffscreenSurface, HostSurface, NativeChildSurface, SurfaceDelegate, SurfaceError,
    SurfaceFrameInfo, SurfaceFrameLike, SurfaceMode, SurfacePoint, SurfaceSize, SurfaceTarget,
    SurfaceViewport,
};

#[cfg(all(feature = "servo", target_os = "macos"))]
/// AppKit child-view helpers for macOS hosts that own their windows and layout.
pub mod macos;

#[cfg(all(
    feature = "servo",
    any(
        target_os = "android",
        target_os = "macos",
        target_os = "windows",
        target_os = "linux"
    )
))]
pub use crate::surface_host::{
    MemoryClipboard, SurfaceClipboard, SurfaceEventLoopWaker, SurfaceFrame, SurfaceFrameImage,
    SurfaceHost, SurfaceHostOptions,
};
