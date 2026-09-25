//! macOS host adapter for create-time Servo or system-WebView selection.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

use dpi::{PhysicalPosition, PhysicalSize};
use objc2::{rc::Retained, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView};
use raw_window_handle::{
    AppKitWindowHandle, HandleError, HasWindowHandle, RawWindowHandle, WindowHandle,
};
use servokit_embedder::{
    ConfigurableHost, ContextMenuAction, Host, HostError, HostEvent, HostInputEvent,
    LoadStatusKind, PopupRequestPolicy, SessionHandle, WebViewCommand, WebViewHandle,
};
use servokit_host::{
    HostSurface, NativeSurface, SurfaceDelegate, SurfaceError, SurfaceSize, SurfaceViewport,
};
use wry::{
    NewWindowResponse, PageLoadEvent, PermissionResponse, Rect, WebView, WebViewBuilder,
    WebViewBuilderExtDarwin, WebViewExtMacOS,
};

use crate::surface::{SurfaceFrame, SurfaceHost, SurfaceHostOptions};

/// Stable WKWebView website-data store identity supplied by the host.
///
/// The identifier belongs only to WebKit. It is not a Servo profile and does
/// not imply storage or credential transfer between engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WebKitDataStoreIdentifier(pub [u8; 16]);

/// macOS engine view selected when a runtime view is created.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MacOsWebViewKind {
    /// Create the existing Servo-backed view.
    #[default]
    Servo,
    /// Create a WRY-owned child WKWebView.
    SystemWebView,
}

/// Create-time options for [`MacOsViewHost`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MacOsWebViewOptions {
    /// Selects the concrete engine view for this handle.
    pub kind: MacOsWebViewKind,
}

impl MacOsWebViewOptions {
    /// Creates options for one concrete macOS engine view.
    pub fn new(kind: MacOsWebViewKind) -> Self {
        Self { kind }
    }
}

/// Construction options for [`MacOsViewHost`].
pub struct MacOsViewHostOptions {
    surface_host: SurfaceHostOptions,
    webkit_data_store_identifier: WebKitDataStoreIdentifier,
}

impl MacOsViewHostOptions {
    /// Combines the existing Servo surface-host inputs with a WebKit-only store identity.
    pub fn new(
        surface_host: SurfaceHostOptions,
        webkit_data_store_identifier: WebKitDataStoreIdentifier,
    ) -> Self {
        Self {
            surface_host,
            webkit_data_store_identifier,
        }
    }
}

/// Supplies the app-owned AppKit parent view used by a system-WebView attachment.
///
/// The returned [`NativeSurface`] is borrowed from the host. ServoKit creates a
/// WRY/WKWebView child inside it and never takes ownership of the app window.
pub trait AppKitViewDelegate: SurfaceDelegate {
    /// Returns the live AppKit parent for this app-owned surface slot.
    fn appkit_parent(
        &mut self,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<NativeSurface<'_>, SurfaceError>;
}

type CallbackTarget = (u64, u64);

#[derive(Default)]
struct SystemCallbacks {
    live: HashSet<CallbackTarget>,
    events: HashMap<CallbackTarget, VecDeque<HostEvent>>,
}

#[derive(Clone)]
struct SystemCallbackSink {
    target: CallbackTarget,
    callbacks: Rc<RefCell<SystemCallbacks>>,
}

impl SystemCallbackSink {
    fn register(
        callbacks: Rc<RefCell<SystemCallbacks>>,
        webview: WebViewHandle,
        generation: u64,
    ) -> Self {
        let target = (webview.raw(), generation);
        let mut state = callbacks.borrow_mut();
        state.live.insert(target);
        state.events.entry(target).or_default();
        drop(state);
        Self { target, callbacks }
    }

    #[cfg(test)]
    fn register_raw(
        callbacks: Rc<RefCell<SystemCallbacks>>,
        webview: u64,
        generation: u64,
    ) -> Self {
        let target = (webview, generation);
        let mut state = callbacks.borrow_mut();
        state.live.insert(target);
        state.events.entry(target).or_default();
        drop(state);
        Self { target, callbacks }
    }

    fn push(&self, event: HostEvent) {
        let mut callbacks = self.callbacks.borrow_mut();
        if callbacks.live.contains(&self.target) {
            callbacks
                .events
                .entry(self.target)
                .or_default()
                .push_back(event);
        }
    }

    fn drain(&self) -> Vec<HostEvent> {
        self.callbacks
            .borrow_mut()
            .events
            .get_mut(&self.target)
            .map(|events| events.drain(..).collect())
            .unwrap_or_default()
    }

    fn clear(&self) {
        if let Some(events) = self.callbacks.borrow_mut().events.get_mut(&self.target) {
            events.clear();
        }
    }

    fn invalidate(&self) {
        let mut callbacks = self.callbacks.borrow_mut();
        callbacks.live.remove(&self.target);
        callbacks.events.remove(&self.target);
    }
}

enum MacOsView {
    Servo,
    System(Box<SystemWebViewState>),
}

struct SystemWebViewState {
    generation: u64,
    webview: Option<WebView>,
    container: Option<AppKitContainer>,
    pending_commands: VecDeque<WebViewCommand>,
    viewport: Option<SurfaceViewport>,
    last_url: Option<String>,
    last_navigation_state: Option<(bool, bool)>,
}

impl SystemWebViewState {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            webview: None,
            container: None,
            pending_commands: VecDeque::new(),
            viewport: None,
            last_url: None,
            last_navigation_state: None,
        }
    }
}

/// One macOS runtime host that owns both Servo and WRY/WKWebView view lifetimes.
///
/// The application still owns its top-level window, layout slot, chrome, tabs,
/// and engine-selection policy. Each [`WebViewHandle`] identifies exactly one
/// concrete engine-view lifetime.
pub struct MacOsViewHost<S> {
    servo: SurfaceHost<S>,
    webkit_data_store_identifier: WebKitDataStoreIdentifier,
    views: HashMap<WebViewHandle, MacOsView>,
    callbacks: Rc<RefCell<SystemCallbacks>>,
    next_generation: u64,
}

impl<S> MacOsViewHost<S> {
    /// Creates the macOS host with existing Servo surface services and one WebKit store.
    pub fn new(services: S, options: MacOsViewHostOptions) -> Self {
        Self {
            servo: SurfaceHost::new(services, options.surface_host),
            webkit_data_store_identifier: options.webkit_data_store_identifier,
            views: HashMap::new(),
            callbacks: Rc::new(RefCell::new(SystemCallbacks::default())),
            next_generation: 0,
        }
    }

    /// Borrows the app-provided surface and AppKit-parent delegate.
    pub fn services(&self) -> &S {
        self.servo.services()
    }

    /// Mutably borrows the app-provided surface and AppKit-parent delegate.
    pub fn services_mut(&mut self) -> &mut S {
        self.servo.services_mut()
    }

    fn next_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1);
        self.next_generation
    }

    fn view_is_servo(&self, webview: WebViewHandle) -> Result<bool, HostError> {
        match self.views.get(&webview) {
            Some(MacOsView::Servo) => Ok(true),
            Some(MacOsView::System(_)) => Ok(false),
            None => Err(unknown_webview(webview)),
        }
    }

    fn system_sink(&self, webview: WebViewHandle, generation: u64) -> SystemCallbackSink {
        SystemCallbackSink {
            target: (webview.raw(), generation),
            callbacks: self.callbacks.clone(),
        }
    }

    fn reset_system_generation(&mut self, webview: WebViewHandle) {
        let old_generation = match self.views.get(&webview) {
            Some(MacOsView::System(state)) => state.generation,
            _ => return,
        };
        self.system_sink(webview, old_generation).invalidate();
        let generation = self.next_generation();
        SystemCallbackSink::register(self.callbacks.clone(), webview, generation);
        if let Some(MacOsView::System(state)) = self.views.get_mut(&webview) {
            state.generation = generation;
            state.last_url = None;
            state.last_navigation_state = None;
        }
    }
}

impl<S> Drop for MacOsViewHost<S> {
    fn drop(&mut self) {
        for (&webview, view) in &self.views {
            if let MacOsView::System(state) = view {
                self.system_sink(webview, state.generation).invalidate();
            }
        }
        // WRY removes each WKWebView from its AppKit superview on drop. Do that
        // while the delegate owning those parent views is still alive.
        self.views.clear();
    }
}

impl<S> ConfigurableHost for MacOsViewHost<S>
where
    S: AppKitViewDelegate + for<'a> SurfaceDelegate<Frame<'a> = SurfaceFrame<'a>>,
{
    type WebViewOptions = MacOsWebViewOptions;

    fn create_webview_with_options(
        &mut self,
        session: SessionHandle,
        webview: WebViewHandle,
        options: Self::WebViewOptions,
    ) -> Result<(), HostError> {
        match options.kind {
            MacOsWebViewKind::Servo => self.create_webview(session, webview),
            MacOsWebViewKind::SystemWebView => {
                if self.views.contains_key(&webview) {
                    return Err(duplicate_webview(webview));
                }
                let generation = self.next_generation();
                SystemCallbackSink::register(self.callbacks.clone(), webview, generation);
                self.views.insert(
                    webview,
                    MacOsView::System(Box::new(SystemWebViewState::new(generation))),
                );
                Ok(())
            }
        }
    }
}

impl<S> Host for MacOsViewHost<S>
where
    S: AppKitViewDelegate + for<'a> SurfaceDelegate<Frame<'a> = SurfaceFrame<'a>>,
{
    fn shutdown(&mut self) -> Result<(), HostError> {
        for (&webview, view) in &self.views {
            if let MacOsView::System(state) = view {
                self.system_sink(webview, state.generation).invalidate();
            }
        }
        self.views
            .retain(|_, view| matches!(view, MacOsView::Servo));
        self.servo.shutdown()?;
        self.views.clear();
        Ok(())
    }

    fn create_webview(
        &mut self,
        session: SessionHandle,
        webview: WebViewHandle,
    ) -> Result<(), HostError> {
        if self.views.contains_key(&webview) {
            return Err(duplicate_webview(webview));
        }
        self.servo.create_webview(session, webview)?;
        self.views.insert(webview, MacOsView::Servo);
        Ok(())
    }

    fn destroy_webview(&mut self, webview: WebViewHandle) -> Result<(), HostError> {
        if self.view_is_servo(webview)? {
            self.servo.destroy_webview(webview)?;
            self.views.remove(&webview);
            return Ok(());
        }

        let generation = match self.views.get(&webview) {
            Some(MacOsView::System(state)) => state.generation,
            _ => return Err(unknown_webview(webview)),
        };
        self.system_sink(webview, generation).invalidate();
        self.views.remove(&webview);
        Ok(())
    }

    fn perform_all_updates(
        &mut self,
        webviews: &[WebViewHandle],
    ) -> Result<Vec<(WebViewHandle, Result<Vec<HostEvent>, HostError>)>, HostError> {
        let servo_views: Vec<_> = webviews
            .iter()
            .copied()
            .filter(|webview| matches!(self.views.get(webview), Some(MacOsView::Servo)))
            .collect();
        let mut servo_results: HashMap<_, _> = self
            .servo
            .perform_all_updates(&servo_views)?
            .into_iter()
            .collect();

        Ok(webviews
            .iter()
            .copied()
            .map(|webview| {
                let result = match self.views.get(&webview) {
                    Some(MacOsView::Servo) => with_navigation_state(
                        servo_results
                            .remove(&webview)
                            .unwrap_or_else(|| Err(unknown_webview(webview))),
                    ),
                    Some(MacOsView::System(_)) => {
                        collect_system_updates(&mut self.views, &self.callbacks, webview)
                    }
                    None => Err(unknown_webview(webview)),
                };
                (webview, result)
            })
            .collect())
    }

    fn attach_surface(
        &mut self,
        webview: WebViewHandle,
        surface: &HostSurface,
        size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.attach_surface_with_viewport(webview, surface, SurfaceViewport::new(size, 1.0))
    }

    fn attach_surface_with_viewport(
        &mut self,
        webview: WebViewHandle,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(
                self.servo
                    .attach_surface_with_viewport(webview, surface, viewport),
            );
        }
        attach_system_view(self, webview, surface, viewport)
    }

    fn resize_surface(
        &mut self,
        webview: WebViewHandle,
        size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(self.servo.resize_surface(webview, size));
        }
        let viewport = system_viewport(&self.views, webview)?.with_size(size);
        update_system_viewport(&mut self.views, webview, viewport)
    }

    fn set_surface_scale_factor(
        &mut self,
        webview: WebViewHandle,
        scale_factor: f32,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(
                self.servo.set_surface_scale_factor(webview, scale_factor),
            );
        }
        let viewport = system_viewport(&self.views, webview)?.with_scale_factor(scale_factor);
        update_system_viewport(&mut self.views, webview, viewport)
    }

    fn update_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        viewport: SurfaceViewport,
        previous_viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(self.servo.update_surface_viewport(
                webview,
                viewport,
                previous_viewport,
            ));
        }
        update_system_viewport(&mut self.views, webview, viewport)
    }

    fn detach_surface(&mut self, webview: WebViewHandle) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(self.servo.detach_surface(webview));
        }
        let state = system_state_mut(&mut self.views, webview)?;
        let native = state.webview.as_ref().ok_or_else(|| {
            HostError::new(format!(
                "system webview {} has no attached WKWebView",
                webview.raw()
            ))
        })?;
        native.set_visible(false).map_err(wry_error)?;
        state
            .container
            .as_ref()
            .expect("attached system webview has a container")
            .detach();
        state.viewport = None;
        Ok(Vec::new())
    }

    fn set_popup_policy(
        &mut self,
        webview: WebViewHandle,
        popup_policy: PopupRequestPolicy,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(self.servo.set_popup_policy(webview, popup_policy));
        }
        if popup_policy == PopupRequestPolicy::DefaultDeny {
            Ok(Vec::new())
        } else {
            Err(system_unsupported(webview, "managed child popup policy"))
        }
    }

    fn attach_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.attach_managed_child_surface(
                webview,
                child_webview_id,
                surface,
                viewport,
            ))
        } else {
            Err(system_unsupported(webview, "managed child surfaces"))
        }
    }

    fn resize_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.resize_managed_child_surface(
                webview,
                child_webview_id,
                size,
            ))
        } else {
            Err(system_unsupported(webview, "managed child surfaces"))
        }
    }

    fn update_managed_child_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        viewport: SurfaceViewport,
        previous_viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.update_managed_child_surface_viewport(
                webview,
                child_webview_id,
                viewport,
                previous_viewport,
            ))
        } else {
            Err(system_unsupported(webview, "managed child surfaces"))
        }
    }

    fn detach_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(
                self.servo
                    .detach_managed_child_surface(webview, child_webview_id),
            )
        } else {
            Err(system_unsupported(webview, "managed child surfaces"))
        }
    }

    fn perform_managed_child_updates(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(
                self.servo
                    .perform_managed_child_updates(webview, child_webview_id),
            )
        } else {
            Err(system_unsupported(webview, "managed child surfaces"))
        }
    }

    fn dispatch_webview_command(
        &mut self,
        webview: WebViewHandle,
        command: &WebViewCommand,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(self.servo.dispatch_webview_command(webview, command));
        }
        dispatch_system_command(&mut self.views, &self.callbacks, webview, command)
    }

    fn resolve_navigation_request(
        &mut self,
        webview: WebViewHandle,
        navigation_id: &str,
        allow: bool,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.resolve_navigation_request(
                webview,
                navigation_id,
                allow,
            ))
        } else {
            Err(system_unsupported(
                webview,
                "asynchronous navigation policy resolution",
            ))
        }
    }

    fn resolve_simple_dialog(
        &mut self,
        webview: WebViewHandle,
        dialog_id: &str,
        confirmed: bool,
        prompt_value: Option<&str>,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.resolve_simple_dialog(
                webview,
                dialog_id,
                confirmed,
                prompt_value,
            ))
        } else {
            Err(system_unsupported(webview, "Servo dialog resolution"))
        }
    }

    fn resolve_context_menu(
        &mut self,
        webview: WebViewHandle,
        context_menu_id: &str,
        action: ContextMenuAction,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(
                self.servo
                    .resolve_context_menu(webview, context_menu_id, action),
            )
        } else {
            Err(system_unsupported(webview, "Servo context-menu resolution"))
        }
    }

    fn dismiss_context_menu(
        &mut self,
        webview: WebViewHandle,
        context_menu_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.dismiss_context_menu(webview, context_menu_id))
        } else {
            Err(system_unsupported(webview, "Servo context-menu dismissal"))
        }
    }

    fn evaluate_javascript(
        &mut self,
        webview: WebViewHandle,
        evaluation_id: &str,
        script: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(
                self.servo
                    .evaluate_javascript(webview, evaluation_id, script),
            )
        } else {
            Err(system_unsupported(webview, "JavaScript evaluation"))
        }
    }

    fn dispatch_input_event(
        &mut self,
        webview: WebViewHandle,
        event: &HostInputEvent,
    ) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            return with_navigation_state(self.servo.dispatch_input_event(webview, event));
        }
        Err(HostError::new(format!(
            "system webview {} receives native AppKit input; forwarded HostInputEvent is unsupported",
            webview.raw()
        )))
    }

    fn synthesize_webview_command_events(&self) -> bool {
        false
    }

    fn perform_updates(&mut self, webview: WebViewHandle) -> Result<Vec<HostEvent>, HostError> {
        if self.view_is_servo(webview)? {
            with_navigation_state(self.servo.perform_updates(webview))
        } else {
            collect_system_updates(&mut self.views, &self.callbacks, webview)
        }
    }

    fn observe_webview_events(
        &mut self,
        webview: WebViewHandle,
        events: &[HostEvent],
    ) -> Result<(), HostError> {
        if self.view_is_servo(webview)? {
            self.servo.observe_webview_events(webview, events)
        } else {
            Ok(())
        }
    }

    fn observe_managed_child_webview_events(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        events: &[HostEvent],
    ) -> Result<(), HostError> {
        if self.view_is_servo(webview)? {
            self.servo
                .observe_managed_child_webview_events(webview, child_webview_id, events)
        } else {
            Err(system_unsupported(webview, "managed child events"))
        }
    }
}

fn with_navigation_state(
    events: Result<Vec<HostEvent>, HostError>,
) -> Result<Vec<HostEvent>, HostError> {
    let events = events?;
    let mut result = Vec::with_capacity(events.len());
    for event in events {
        let navigation_state = match &event {
            HostEvent::HistoryChanged {
                can_go_back,
                can_go_forward,
                ..
            } => Some((*can_go_back, *can_go_forward)),
            _ => None,
        };
        result.push(event);
        if let Some((can_go_back, can_go_forward)) = navigation_state {
            result.push(HostEvent::NavigationStateChanged {
                can_go_back,
                can_go_forward,
            });
        }
    }
    Ok(result)
}

fn attach_system_view<S>(
    host: &mut MacOsViewHost<S>,
    webview: WebViewHandle,
    surface: &HostSurface,
    viewport: SurfaceViewport,
) -> Result<Vec<HostEvent>, HostError>
where
    S: AppKitViewDelegate + for<'a> SurfaceDelegate<Frame<'a> = SurfaceFrame<'a>>,
{
    let (generation, pending_commands, has_native_view) = {
        let state = system_state_mut(&mut host.views, webview)?;
        if state.viewport.is_some() {
            return Err(HostError::new(format!(
                "system webview {} already has an attached surface",
                webview.raw()
            )));
        }
        (
            state.generation,
            state.pending_commands.iter().cloned().collect::<Vec<_>>(),
            state.webview.is_some(),
        )
    };
    let sink = host.system_sink(webview, generation);
    let data_store_identifier = host.webkit_data_store_identifier;

    let parent_surface = host
        .servo
        .services_mut()
        .appkit_parent(surface, viewport)
        .map_err(|error| HostError::new(error.to_string()))?;
    let parent = BorrowedAppKitParent(parent_surface.window_handle());

    if has_native_view {
        let mut events = Vec::new();
        let state = system_state_mut(&mut host.views, webview)?;
        let native = state
            .webview
            .as_ref()
            .expect("system webview presence was checked");
        let container = state
            .container
            .as_ref()
            .expect("system webview presence was checked");
        container.attach(&parent)?;
        if let Err(error) = native
            .set_bounds(wry_rect(viewport))
            .and_then(|()| native.set_visible(true))
        {
            container.detach();
            return Err(wry_error(error));
        }
        for command in &pending_commands {
            match apply_system_command(native, command) {
                Ok(command_events) => events.extend(command_events),
                Err(error) => {
                    let _ = native.set_visible(false);
                    container.detach();
                    return Err(error);
                }
            }
        }
        state.pending_commands.clear();
        state.viewport = Some(viewport);
        events.extend(collect_system_updates(
            &mut host.views,
            &host.callbacks,
            webview,
        )?);
        return Ok(events);
    }

    let initial_url = pending_commands
        .first()
        .and_then(|command| match command {
            WebViewCommand::LoadUrl(request) => Some(request.url.as_str()),
            _ => None,
        })
        .unwrap_or("about:blank");
    let promoted_initial_load =
        matches!(pending_commands.first(), Some(WebViewCommand::LoadUrl(_)));

    let container = AppKitContainer::new(&parent)?;
    let built = {
        build_system_webview(
            &container,
            viewport,
            initial_url,
            data_store_identifier,
            sink.clone(),
        )
    };

    let native = match built {
        Ok(native) => native,
        Err(error) => {
            sink.clear();
            host.reset_system_generation(webview);
            return Err(error);
        }
    };

    let mut events = Vec::new();
    for command in pending_commands
        .iter()
        .skip(usize::from(promoted_initial_load))
    {
        match apply_system_command(&native, command) {
            Ok(command_events) => events.extend(command_events),
            Err(error) => {
                drop(native);
                sink.clear();
                host.reset_system_generation(webview);
                return Err(error);
            }
        }
    }

    let state = system_state_mut(&mut host.views, webview)?;
    state.webview = Some(native);
    state.container = Some(container);
    state.pending_commands.clear();
    state.viewport = Some(viewport);
    events.extend(collect_system_updates(
        &mut host.views,
        &host.callbacks,
        webview,
    )?);
    Ok(events)
}

fn build_system_webview(
    parent: &impl HasWindowHandle,
    viewport: SurfaceViewport,
    initial_url: &str,
    data_store_identifier: WebKitDataStoreIdentifier,
    sink: SystemCallbackSink,
) -> Result<WebView, HostError> {
    let title_sink = sink.clone();
    let load_sink = sink.clone();
    let crash_sink = sink;
    WebViewBuilder::new()
        .with_url(initial_url)
        .with_bounds(wry_rect(viewport))
        .with_accept_first_mouse(true)
        .with_data_store_identifier(data_store_identifier.0)
        .with_navigation_handler(|_| true)
        .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
        .with_download_started_handler(|_, _| false)
        .with_permission_handler(|_| PermissionResponse::Default)
        .with_document_title_changed_handler(move |title| {
            title_sink.push(HostEvent::PageTitleChanged {
                title: (!title.is_empty()).then_some(title),
            });
        })
        .with_on_page_load_handler(move |event, _| {
            let status = match event {
                PageLoadEvent::Started => LoadStatusKind::Started,
                PageLoadEvent::Finished => LoadStatusKind::Complete,
            };
            load_sink.push(HostEvent::LoadStatusChanged { status });
        })
        .with_on_web_content_process_terminate_handler(move || {
            crash_sink.push(HostEvent::Crashed {
                url: None,
                reason: "WebKit web content process terminated".to_owned(),
                backtrace: None,
            });
        })
        .build_as_child(parent)
        .map_err(wry_error)
}

fn dispatch_system_command(
    views: &mut HashMap<WebViewHandle, MacOsView>,
    callbacks: &Rc<RefCell<SystemCallbacks>>,
    webview: WebViewHandle,
    command: &WebViewCommand,
) -> Result<Vec<HostEvent>, HostError> {
    let state = system_state_mut(views, webview)?;
    if should_queue_system_command(state.webview.is_some(), state.viewport.is_some(), command) {
        state.pending_commands.push_back(command.clone());
        return Ok(Vec::new());
    }
    let native = state
        .webview
        .as_ref()
        .expect("an attached system view has a native webview");

    let mut events = apply_system_command(native, command)?;
    events.extend(collect_system_updates(views, callbacks, webview)?);
    Ok(events)
}

fn should_queue_system_command(
    has_native_view: bool,
    has_surface: bool,
    command: &WebViewCommand,
) -> bool {
    !has_native_view
        || !has_surface && matches!(command, WebViewCommand::Focus | WebViewCommand::Blur)
}

fn apply_system_command(
    native: &WebView,
    command: &WebViewCommand,
) -> Result<Vec<HostEvent>, HostError> {
    let event = match command {
        WebViewCommand::LoadUrl(request) => {
            native.load_url(&request.url).map_err(wry_error)?;
            None
        }
        WebViewCommand::Reload => {
            native.reload().map_err(wry_error)?;
            None
        }
        WebViewCommand::GoBack => {
            native.go_back().map_err(wry_error)?;
            None
        }
        WebViewCommand::GoForward => {
            native.go_forward().map_err(wry_error)?;
            None
        }
        WebViewCommand::Focus => {
            native.focus().map_err(wry_error)?;
            Some(HostEvent::FocusChanged { is_focused: true })
        }
        WebViewCommand::Blur => {
            blur_system_webview(native)?;
            Some(HostEvent::FocusChanged { is_focused: false })
        }
    };
    Ok(event.into_iter().collect())
}

fn blur_system_webview(native: &WebView) -> Result<(), HostError> {
    let webview = native.webview();
    // SAFETY: WRY retains the WKWebView's ServoKit-owned container, and an
    // attached container's superview is the current app-owned parent.
    let container = unsafe { webview.superview() };
    let parent = container
        .as_ref()
        .and_then(|container| unsafe { container.superview() });
    let Some(window) = parent.as_ref().and_then(|parent| parent.window()) else {
        return native.focus_parent().map_err(wry_error);
    };
    if !window.makeFirstResponder(parent.as_deref().map(|view| &**view)) {
        window.makeFirstResponder(None);
    }
    Ok(())
}

fn collect_system_updates(
    views: &mut HashMap<WebViewHandle, MacOsView>,
    callbacks: &Rc<RefCell<SystemCallbacks>>,
    webview: WebViewHandle,
) -> Result<Vec<HostEvent>, HostError> {
    let generation = match views.get(&webview) {
        Some(MacOsView::System(state)) => state.generation,
        Some(MacOsView::Servo) => {
            return Err(HostError::new(format!(
                "webview {} is Servo-backed",
                webview.raw()
            )))
        }
        None => return Err(unknown_webview(webview)),
    };
    let sink = SystemCallbackSink {
        target: (webview.raw(), generation),
        callbacks: callbacks.clone(),
    };
    let mut events = sink.drain();
    let state = system_state_mut(views, webview)?;
    let Some(native) = state.webview.as_ref() else {
        return Ok(events);
    };

    let url = native.url().map_err(wry_error)?;
    if state.last_url.as_deref() != Some(url.as_str()) {
        state.last_url = Some(url.clone());
        events.push(HostEvent::UrlChanged { url });
    }

    let navigation_state = (
        native.can_go_back().map_err(wry_error)?,
        native.can_go_forward().map_err(wry_error)?,
    );
    if state.last_navigation_state != Some(navigation_state) {
        state.last_navigation_state = Some(navigation_state);
        events.push(HostEvent::NavigationStateChanged {
            can_go_back: navigation_state.0,
            can_go_forward: navigation_state.1,
        });
    }
    Ok(events)
}

fn update_system_viewport(
    views: &mut HashMap<WebViewHandle, MacOsView>,
    webview: WebViewHandle,
    viewport: SurfaceViewport,
) -> Result<Vec<HostEvent>, HostError> {
    let state = system_state_mut(views, webview)?;
    let native = state.webview.as_ref().ok_or_else(|| {
        HostError::new(format!(
            "system webview {} has no attached WKWebView",
            webview.raw()
        ))
    })?;
    native.set_bounds(wry_rect(viewport)).map_err(wry_error)?;
    state.viewport = Some(viewport);
    Ok(Vec::new())
}

fn system_viewport(
    views: &HashMap<WebViewHandle, MacOsView>,
    webview: WebViewHandle,
) -> Result<SurfaceViewport, HostError> {
    match views.get(&webview) {
        Some(MacOsView::System(state)) => state.viewport.ok_or_else(|| {
            HostError::new(format!(
                "system webview {} does not have an attached surface",
                webview.raw()
            ))
        }),
        Some(MacOsView::Servo) => Err(HostError::new(format!(
            "webview {} is Servo-backed",
            webview.raw()
        ))),
        None => Err(unknown_webview(webview)),
    }
}

fn system_state_mut(
    views: &mut HashMap<WebViewHandle, MacOsView>,
    webview: WebViewHandle,
) -> Result<&mut SystemWebViewState, HostError> {
    match views.get_mut(&webview) {
        Some(MacOsView::System(state)) => Ok(state),
        Some(MacOsView::Servo) => Err(HostError::new(format!(
            "webview {} is Servo-backed",
            webview.raw()
        ))),
        None => Err(unknown_webview(webview)),
    }
}

fn wry_rect(viewport: SurfaceViewport) -> Rect {
    Rect {
        position: PhysicalPosition::new(viewport.origin.x, viewport.origin.y).into(),
        size: PhysicalSize::new(viewport.size.width, viewport.size.height).into(),
    }
}

fn unknown_webview(webview: WebViewHandle) -> HostError {
    HostError::new(format!("webview {} is not registered", webview.raw()))
}

fn duplicate_webview(webview: WebViewHandle) -> HostError {
    HostError::new(format!("webview {} is already registered", webview.raw()))
}

fn system_unsupported(webview: WebViewHandle, capability: &str) -> HostError {
    HostError::new(format!(
        "system webview {} does not support {capability}",
        webview.raw()
    ))
}

fn wry_error(error: wry::Error) -> HostError {
    HostError::new(error.to_string())
}

struct BorrowedAppKitParent<'a>(WindowHandle<'a>);

impl HasWindowHandle for BorrowedAppKitParent<'_> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(self.0)
    }
}

struct AppKitContainer(Retained<NSView>);

impl AppKitContainer {
    fn new(parent: &impl HasWindowHandle) -> Result<Self, HostError> {
        let parent = appkit_parent_view(parent)?;
        let main_thread = MainThreadMarker::new().ok_or_else(|| {
            HostError::new("system webview must attach on the AppKit main thread")
        })?;
        let view = NSView::initWithFrame(NSView::alloc(main_thread), parent.bounds());
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        parent.addSubview(&view);
        Ok(Self(view))
    }

    fn attach(&self, parent: &impl HasWindowHandle) -> Result<(), HostError> {
        let parent = appkit_parent_view(parent)?;
        self.0.setFrame(parent.bounds());
        parent.addSubview(&self.0);
        Ok(())
    }

    fn detach(&self) {
        self.0.removeFromSuperview();
    }
}

impl Drop for AppKitContainer {
    fn drop(&mut self) {
        self.detach();
    }
}

impl HasWindowHandle for AppKitContainer {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = AppKitWindowHandle::new(NonNull::from(&*self.0).cast::<c_void>());
        // SAFETY: the returned handle borrows the retained NSView owned by self.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(handle)) })
    }
}

fn appkit_parent_view(parent: &impl HasWindowHandle) -> Result<&NSView, HostError> {
    let handle = parent
        .window_handle()
        .map_err(|error| HostError::new(error.to_string()))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err(HostError::new(
            "system webview requires an AppKit parent view",
        ));
    };
    // SAFETY: the app guarantees that the borrowed raw handle identifies a
    // live NSView for the duration of this call on AppKit's owning thread.
    Ok(unsafe { handle.ns_view.cast::<NSView>().as_ref() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::HostInputEvent;
    use crate::runtime::{Runtime, ServokitError};
    use crate::surface::SurfaceTarget;

    struct NoopDelegate;

    impl SurfaceDelegate for NoopDelegate {
        type Frame<'a> = SurfaceFrame<'a>;

        fn render_target(
            &mut self,
            _surface: &HostSurface,
            _viewport: SurfaceViewport,
        ) -> Result<SurfaceTarget<'_>, SurfaceError> {
            Err(SurfaceError::new("not used by this test"))
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

    impl AppKitViewDelegate for NoopDelegate {
        fn appkit_parent(
            &mut self,
            _surface: &HostSurface,
            _viewport: SurfaceViewport,
        ) -> Result<NativeSurface<'_>, SurfaceError> {
            Err(SurfaceError::new("not used by this test"))
        }
    }

    fn runtime() -> Runtime<MacOsViewHost<NoopDelegate>> {
        Runtime::new(MacOsViewHost::new(
            NoopDelegate,
            MacOsViewHostOptions::new(
                SurfaceHostOptions::default(),
                WebKitDataStoreIdentifier([0x34; 16]),
            ),
        ))
    }

    #[test]
    fn system_webview_rejects_forwarded_host_input() {
        let mut runtime = runtime();
        let session = runtime.create_session();
        let webview = runtime
            .create_webview_with_options(
                session,
                MacOsWebViewOptions::new(MacOsWebViewKind::SystemWebView),
            )
            .unwrap();

        let error = runtime
            .dispatch_input_event(webview, HostInputEvent::Focus { is_focused: true })
            .unwrap_err();
        assert!(matches!(error, ServokitError::Host(_)));
        assert!(error.to_string().contains("receives native AppKit input"));
    }

    #[test]
    fn system_webview_discards_callbacks_after_destroy_and_replacement() {
        let callbacks = Rc::new(RefCell::new(SystemCallbacks::default()));
        let first = SystemCallbackSink::register_raw(callbacks.clone(), 1, 1);
        first.push(HostEvent::FocusChanged { is_focused: true });
        assert_eq!(first.drain().len(), 1);

        first.invalidate();
        let replacement = SystemCallbackSink::register_raw(callbacks, 1, 2);
        first.push(HostEvent::FocusChanged { is_focused: false });
        replacement.push(HostEvent::FocusChanged { is_focused: true });

        assert!(first.drain().is_empty());
        assert_eq!(
            replacement.drain(),
            vec![HostEvent::FocusChanged { is_focused: true }]
        );
    }

    #[test]
    fn detached_system_focus_and_blur_wait_for_reattach() {
        assert!(should_queue_system_command(
            true,
            false,
            &WebViewCommand::Focus
        ));
        assert!(should_queue_system_command(
            true,
            false,
            &WebViewCommand::Blur
        ));
        assert!(!should_queue_system_command(
            true,
            true,
            &WebViewCommand::Focus
        ));
    }

    #[test]
    fn default_creation_remains_servo_backed() {
        let mut runtime = runtime();
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        runtime
            .dispatch_input_event(webview, HostInputEvent::Focus { is_focused: true })
            .unwrap();
    }

    #[test]
    fn servo_history_also_reports_the_common_navigation_state() {
        let events = with_navigation_state(Ok(vec![HostEvent::HistoryChanged {
            entries: vec!["https://example.test/".to_owned()],
            current: 0,
            can_go_back: false,
            can_go_forward: true,
        }]))
        .unwrap();

        assert!(matches!(
            events.as_slice(),
            [
                HostEvent::HistoryChanged { .. },
                HostEvent::NavigationStateChanged {
                    can_go_back: false,
                    can_go_forward: true
                }
            ]
        ));
    }
}
