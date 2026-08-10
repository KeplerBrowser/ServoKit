use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

use dpi::PhysicalSize;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, DisplayHandle, RawDisplayHandle, RawWindowHandle,
    WindowHandle,
};
use servo::{
    ClipboardDelegate, EventLoopWaker, RefreshDriver, RenderingContext, StringRequest, WebView,
    WindowRenderingContext,
};

use servokit_embedder::{
    ContextMenuAction, HostEvent, ImeCompositionState, KeyboardKey, NavigationRequest,
    PopupRequestPolicy, ServoWebView, ServoWebViewInit, TouchEventKind, WebViewCommand,
};

use crate::jni_bridge::{
    android_platform_clipboard_clear_text, android_platform_clipboard_get_text,
    android_platform_clipboard_set_text,
};
use crate::{AndroidRenderBackend, NativeWindowHandle, SurfaceSize};

pub(crate) struct ServoAndroidBackend {
    refresh_driver: Rc<VsyncRefreshDriver>,
    webview: ServoWebView,
    rendering_context: Rc<WindowRenderingContext>,
    surface_attached: bool,
}

impl ServoAndroidBackend {
    pub(crate) fn new(
        native_window: NativeWindowHandle,
        size: SurfaceSize,
        density: f32,
        initial_url: Option<&str>,
    ) -> Result<Self, String> {
        let refresh_driver = Rc::new(VsyncRefreshDriver::default());
        let rendering_context = Rc::new(create_rendering_context(
            native_window,
            size,
            refresh_driver.clone(),
        )?);
        let webview = ServoWebView::new(ServoWebViewInit {
            rendering_context: rendering_context.clone(),
            clipboard_delegate: Rc::new(AndroidClipboardDelegate::default()),
            event_loop_waker: Box::new(PollingEventLoopWaker),
            initial_url: initial_url.map(str::to_owned),
            density,
            popup_policy: PopupRequestPolicy::DefaultDeny,
            managed_child_rendering_context_factory: None,
        })?;

        Ok(Self {
            refresh_driver,
            webview,
            rendering_context,
            surface_attached: true,
        })
    }

    pub(crate) fn attach_surface(
        &mut self,
        native_window: NativeWindowHandle,
        size: SurfaceSize,
        density: f32,
    ) -> Result<Vec<HostEvent>, String> {
        if self.surface_attached {
            return Ok(Vec::new());
        }

        let window_handle = window_handle_from_native_window(native_window)?;
        self.rendering_context
            .set_window(window_handle, physical_size(size))
            .map_err(|error| format!("{error:?}"))?;
        self.surface_attached = true;
        self.webview.set_hidpi_scale_factor(density);
        self.webview.resize(size);
        self.webview.request_paint();
        self.perform_updates()
    }

    pub(crate) fn resize_surface(
        &mut self,
        size: SurfaceSize,
        density: f32,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.set_hidpi_scale_factor(density);
        self.webview.resize(size);
        self.webview.request_paint();
        self.perform_updates()
    }

    pub(crate) fn detach_surface(&mut self) -> Result<Vec<HostEvent>, String> {
        if !self.surface_attached {
            return Ok(Vec::new());
        }

        self.rendering_context
            .take_window()
            .map_err(|error| format!("{error:?}"))?;
        self.surface_attached = false;
        Ok(self.webview.drain_events())
    }

    pub(crate) fn load_url(&mut self, url: &str) -> Result<Vec<HostEvent>, String> {
        let request = NavigationRequest::new(url).map_err(|error| error.to_string())?;
        self.webview
            .dispatch_command(WebViewCommand::LoadUrl(request))?;
        self.perform_updates()
    }

    pub(crate) fn reload(&mut self) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_command(WebViewCommand::Reload)?;
        self.perform_updates()
    }

    pub(crate) fn go_back(&mut self) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_command(WebViewCommand::GoBack)?;
        self.perform_updates()
    }

    pub(crate) fn go_forward(&mut self) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_command(WebViewCommand::GoForward)?;
        self.perform_updates()
    }

    pub(crate) fn focus(&mut self) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_command(WebViewCommand::Focus)?;
        self.perform_updates()
    }

    pub(crate) fn blur(&mut self) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_command(WebViewCommand::Blur)?;
        self.perform_updates()
    }

    pub(crate) fn evaluate_javascript(
        &mut self,
        evaluation_id: &str,
        script: &str,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.evaluate_javascript(evaluation_id, script)?;
        self.perform_updates()
    }

    pub(crate) fn dispatch_touch_event(
        &mut self,
        event_kind: TouchEventKind,
        touch_id: i32,
        x: f32,
        y: f32,
    ) -> Result<Vec<HostEvent>, String> {
        if !self.surface_attached {
            return Ok(Vec::new());
        }

        self.webview
            .dispatch_touch_event(event_kind, touch_id, x, y)?;
        self.perform_updates()
    }

    pub(crate) fn trigger_context_menu(
        &mut self,
        x: f32,
        y: f32,
    ) -> Result<Vec<HostEvent>, String> {
        if !self.surface_attached {
            return Ok(Vec::new());
        }

        self.webview.trigger_context_menu(x, y)?;
        self.perform_updates()
    }

    pub(crate) fn dispatch_ime_composition(
        &mut self,
        state: ImeCompositionState,
        text: &str,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_ime_composition(state, text)?;
        self.perform_updates()
    }

    pub(crate) fn dismiss_input_method(&mut self) -> Result<Vec<HostEvent>, String> {
        self.webview.dismiss_input_method()?;
        self.perform_updates()
    }

    pub(crate) fn dispatch_keyboard_key(
        &mut self,
        key: KeyboardKey,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.dispatch_keyboard_key(key)?;
        self.perform_updates()
    }

    pub(crate) fn resolve_select_element(
        &mut self,
        select_element_id: &str,
        selected_options: Vec<usize>,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview
            .resolve_select_element(select_element_id, selected_options)?;
        self.perform_updates()
    }

    pub(crate) fn resolve_file_picker(
        &mut self,
        file_picker_id: &str,
        selected_paths: Vec<String>,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview
            .resolve_file_picker(file_picker_id, selected_paths)?;
        self.perform_updates()
    }

    pub(crate) fn dismiss_file_picker(
        &mut self,
        file_picker_id: &str,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.dismiss_file_picker(file_picker_id)?;
        self.perform_updates()
    }

    pub(crate) fn resolve_context_menu(
        &mut self,
        context_menu_id: &str,
        action: ContextMenuAction,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.resolve_context_menu(context_menu_id, action)?;
        self.perform_updates()
    }

    pub(crate) fn dismiss_context_menu(
        &mut self,
        context_menu_id: &str,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview.dismiss_context_menu(context_menu_id)?;
        self.perform_updates()
    }

    pub(crate) fn has_pending_permission_request(&self) -> bool {
        self.webview.has_pending_permission_request()
    }

    pub(crate) fn pending_permission_request(&self) -> Option<(String, String)> {
        self.webview.pending_permission_request()
    }

    pub(crate) fn resolve_permission(&mut self, allow: bool) -> Result<Vec<HostEvent>, String> {
        self.webview.resolve_permission(allow)?;
        self.perform_updates()
    }

    pub(crate) fn resolve_navigation_request(
        &mut self,
        navigation_id: &str,
        allow: bool,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview
            .resolve_navigation_request(navigation_id, allow)?;
        self.perform_updates()
    }

    pub(crate) fn perform_updates(&mut self) -> Result<Vec<HostEvent>, String> {
        let refresh_driver = self.refresh_driver.clone();
        let rendering_context = self.rendering_context.clone();
        let surface_attached = self.surface_attached;
        self.webview.perform_updates(
            surface_attached,
            move || refresh_driver.notify_vsync(),
            move || {
                rendering_context.present();
            },
        )
    }

    pub(crate) fn resolve_simple_dialog(
        &mut self,
        dialog_id: &str,
        confirmed: bool,
        prompt_value: Option<&str>,
    ) -> Result<Vec<HostEvent>, String> {
        self.webview
            .resolve_simple_dialog(dialog_id, confirmed, prompt_value)?;
        self.perform_updates()
    }

    pub(crate) fn shutdown(&mut self) {
        if self.surface_attached {
            let _ = self.rendering_context.take_window();
            self.surface_attached = false;
        }
    }
}

#[derive(Default)]
struct AndroidClipboardDelegate {
    fallback_text: RefCell<String>,
}

impl ClipboardDelegate for AndroidClipboardDelegate {
    fn clear(&self, _webview: WebView) {
        self.fallback_text.borrow_mut().clear();
        let _ = android_platform_clipboard_clear_text();
    }

    fn get_text(&self, _webview: WebView, request: StringRequest) {
        match android_platform_clipboard_get_text() {
            Ok(Some(text)) => {
                self.fallback_text.replace(text.clone());
                request.success(text);
            }
            Ok(None) => request.success(self.fallback_text.borrow().clone()),
            Err(_) => request.success(self.fallback_text.borrow().clone()),
        }
    }

    fn set_text(&self, _webview: WebView, new_contents: String) {
        self.fallback_text.replace(new_contents.clone());
        let _ = android_platform_clipboard_set_text(&new_contents);
    }
}

#[derive(Default)]
struct VsyncRefreshDriver {
    start_frame_callbacks: RefCell<Vec<Box<dyn Fn() + Send>>>,
}

impl VsyncRefreshDriver {
    fn notify_vsync(&self) {
        let start_frame_callbacks: Vec<_> =
            self.start_frame_callbacks.borrow_mut().drain(..).collect();
        for callback in start_frame_callbacks {
            callback();
        }
    }
}

impl RefreshDriver for VsyncRefreshDriver {
    fn observe_next_frame(&self, new_start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.start_frame_callbacks
            .borrow_mut()
            .push(new_start_frame_callback);
    }
}

#[derive(Clone, Copy, Default)]
struct PollingEventLoopWaker;

impl EventLoopWaker for PollingEventLoopWaker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(*self)
    }

    fn wake(&self) {}
}

fn create_rendering_context(
    native_window: NativeWindowHandle,
    size: SurfaceSize,
    refresh_driver: Rc<VsyncRefreshDriver>,
) -> Result<WindowRenderingContext, String> {
    let raw_display_handle = RawDisplayHandle::Android(AndroidDisplayHandle::new());
    let display_handle = unsafe { DisplayHandle::borrow_raw(raw_display_handle) };
    let window_handle = window_handle_from_native_window(native_window)?;

    WindowRenderingContext::new_with_refresh_driver(
        display_handle,
        window_handle,
        physical_size(size),
        refresh_driver,
    )
    .map_err(|error| format!("{error:?}"))
}

fn window_handle_from_native_window(
    native_window: NativeWindowHandle,
) -> Result<WindowHandle<'static>, String> {
    let window = NonNull::new(native_window).ok_or_else(|| "native window is null".to_owned())?;
    let raw_window_handle = RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(window.cast()));

    Ok(unsafe { WindowHandle::borrow_raw(raw_window_handle) })
}

fn physical_size(size: SurfaceSize) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width, size.height)
}

impl AndroidRenderBackend for ServoAndroidBackend {
    fn attach_surface(
        &mut self,
        native_window: NativeWindowHandle,
        size: SurfaceSize,
        density: f32,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::attach_surface(self, native_window, size, density)
    }

    fn resize_surface(
        &mut self,
        size: SurfaceSize,
        density: f32,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resize_surface(self, size, density)
    }

    fn detach_surface(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::detach_surface(self)
    }

    fn load_url(&mut self, url: &str) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::load_url(self, url)
    }

    fn reload(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::reload(self)
    }

    fn go_back(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::go_back(self)
    }

    fn go_forward(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::go_forward(self)
    }

    fn focus(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::focus(self)
    }

    fn blur(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::blur(self)
    }

    fn evaluate_javascript(
        &mut self,
        evaluation_id: &str,
        script: &str,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::evaluate_javascript(self, evaluation_id, script)
    }

    fn resolve_simple_dialog(
        &mut self,
        dialog_id: &str,
        confirmed: bool,
        prompt_value: Option<&str>,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resolve_simple_dialog(self, dialog_id, confirmed, prompt_value)
    }

    fn dispatch_ime_composition(
        &mut self,
        state: ImeCompositionState,
        text: &str,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::dispatch_ime_composition(self, state, text)
    }

    fn dismiss_input_method(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::dismiss_input_method(self)
    }

    fn dispatch_keyboard_key(&mut self, key: KeyboardKey) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::dispatch_keyboard_key(self, key)
    }

    fn perform_updates(&mut self) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::perform_updates(self)
    }

    fn dispatch_touch_event(
        &mut self,
        event_kind: TouchEventKind,
        touch_id: i32,
        x: f32,
        y: f32,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::dispatch_touch_event(self, event_kind, touch_id, x, y)
    }

    fn trigger_context_menu(&mut self, x: f32, y: f32) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::trigger_context_menu(self, x, y)
    }

    fn resolve_select_element(
        &mut self,
        select_element_id: &str,
        selected_options: Vec<usize>,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resolve_select_element(self, select_element_id, selected_options)
    }

    fn resolve_file_picker(
        &mut self,
        file_picker_id: &str,
        selected_paths: Vec<String>,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resolve_file_picker(self, file_picker_id, selected_paths)
    }

    fn dismiss_file_picker(&mut self, file_picker_id: &str) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::dismiss_file_picker(self, file_picker_id)
    }

    fn has_pending_permission_request(&self) -> bool {
        ServoAndroidBackend::has_pending_permission_request(self)
    }

    fn pending_permission_request(&self) -> Option<(String, String)> {
        ServoAndroidBackend::pending_permission_request(self)
    }

    fn resolve_permission(&mut self, allow: bool) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resolve_permission(self, allow)
    }

    fn resolve_navigation_request(
        &mut self,
        navigation_id: &str,
        allow: bool,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resolve_navigation_request(self, navigation_id, allow)
    }

    fn resolve_context_menu(
        &mut self,
        context_menu_id: &str,
        action: ContextMenuAction,
    ) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::resolve_context_menu(self, context_menu_id, action)
    }

    fn dismiss_context_menu(&mut self, context_menu_id: &str) -> Result<Vec<HostEvent>, String> {
        ServoAndroidBackend::dismiss_context_menu(self, context_menu_id)
    }

    fn shutdown(&mut self) {
        ServoAndroidBackend::shutdown(self);
    }
}
