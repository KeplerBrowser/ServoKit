use crate::NativeWindowHandle;

#[cfg(target_os = "android")]
unsafe extern "C" {
    fn ANativeWindow_release(window: NativeWindowHandle);
}

pub(crate) fn release_native_window(_native_window: Option<NativeWindowHandle>) {
    #[cfg(target_os = "android")]
    if let Some(native_window) = _native_window {
        unsafe { ANativeWindow_release(native_window) };
    }
}
