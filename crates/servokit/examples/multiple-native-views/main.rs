//! Public-facade AppKit proof: independent pages, input, resize, replacement and shutdown.
//! The example owns its native child NSViews; renderer detach precedes native removal.
#![allow(deprecated)]

#[cfg(target_os = "macos")]
mod app;

fn main() {
    #[cfg(target_os = "macos")]
    app::run();
    #[cfg(not(target_os = "macos"))]
    panic!("this native child-surface proof requires macOS");
}
