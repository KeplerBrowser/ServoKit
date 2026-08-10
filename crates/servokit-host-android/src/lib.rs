#![deny(clippy::missing_safety_doc)]
#![deny(clippy::not_unsafe_ptr_arg_deref)]

#[cfg(target_os = "android")]
mod android_backend;
mod android_host;
mod commands;
mod ffi;
#[cfg(target_os = "android")]
mod jni_bridge;
mod native_window;
mod registry;
mod state;
mod token_api;
mod types;

pub(crate) use android_host::AndroidRenderBackend;
pub use android_host::NativeWindowHandle;
pub use ffi::*;
pub use servokit_host::SurfaceSize;
pub use state::HostHandle;
pub use token_api::*;
pub use types::ServoStatus;
