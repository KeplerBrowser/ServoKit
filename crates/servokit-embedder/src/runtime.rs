//! Reusable Servokit runtime/webview ownership for facade and host adapters.
//!
//! This module keeps shared command dispatch, mockable webview state, and
//! event draining behind `servokit-embedder` so facades do not duplicate the
//! Servo-shaped runtime path.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

use crate::{
    ContextMenuAction, HostEvent, HostInputEvent, LoadStatusKind, NavigationError,
    NavigationRequest, PopupRequestPolicy, ServoAdapterState,
};
use servokit_host::{HostSurface, SurfaceSize, SurfaceViewport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionHandle(u64);

impl SessionHandle {
    pub fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WebViewHandle(u64);

impl WebViewHandle {
    pub fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServokitEvent {
    pub webview: WebViewHandle,
    pub managed_child_webview_id: Option<String>,
    pub event: HostEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebViewCommand {
    LoadUrl(NavigationRequest),
    Reload,
    GoBack,
    GoForward,
    Focus,
    Blur,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostCall {
    CreateWebView {
        session: SessionHandle,
        webview: WebViewHandle,
    },
    AttachSurface {
        webview: WebViewHandle,
        surface: HostSurface,
        size: SurfaceSize,
    },
    AttachSurfaceWithViewport {
        webview: WebViewHandle,
        surface: HostSurface,
        viewport: SurfaceViewport,
    },
    ResizeSurface {
        webview: WebViewHandle,
        size: SurfaceSize,
    },
    SetSurfaceScaleFactor {
        webview: WebViewHandle,
        scale_factor: f32,
    },
    UpdateSurfaceViewport {
        webview: WebViewHandle,
        viewport: SurfaceViewport,
    },
    DetachSurface {
        webview: WebViewHandle,
    },
    SetPopupPolicy {
        webview: WebViewHandle,
        popup_policy: PopupRequestPolicy,
    },
    AttachManagedChildSurface {
        webview: WebViewHandle,
        child_webview_id: String,
        surface: HostSurface,
        viewport: SurfaceViewport,
    },
    ResizeManagedChildSurface {
        webview: WebViewHandle,
        child_webview_id: String,
        size: SurfaceSize,
    },
    UpdateManagedChildSurfaceViewport {
        webview: WebViewHandle,
        child_webview_id: String,
        viewport: SurfaceViewport,
    },
    DetachManagedChildSurface {
        webview: WebViewHandle,
        child_webview_id: String,
    },
    PerformManagedChildUpdates {
        webview: WebViewHandle,
        child_webview_id: String,
    },
    DispatchWebViewCommand {
        webview: WebViewHandle,
        command: WebViewCommand,
    },
    ResolveNavigationRequest {
        webview: WebViewHandle,
        navigation_id: String,
        allow: bool,
    },
    PerformUpdates {
        webview: WebViewHandle,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HostError {}

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError {
    UnknownSession(SessionHandle),
    UnknownWebView(WebViewHandle),
    SurfaceAlreadyAttached(WebViewHandle),
    SurfaceNotAttached(WebViewHandle),
    ManagedChildSurfaceAlreadyAttached(WebViewHandle, String),
    ManagedChildSurfaceNotAttached(WebViewHandle, String),
    InvalidUrl(NavigationError),
    Host(HostError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(session) => {
                write!(formatter, "session {} is not registered", session.raw())
            }
            Self::UnknownWebView(webview) => {
                write!(formatter, "webview {} is not registered", webview.raw())
            }
            Self::SurfaceAlreadyAttached(webview) => {
                write!(
                    formatter,
                    "webview {} already has an attached surface",
                    webview.raw()
                )
            }
            Self::SurfaceNotAttached(webview) => {
                write!(
                    formatter,
                    "webview {} does not have an attached surface",
                    webview.raw()
                )
            }
            Self::ManagedChildSurfaceAlreadyAttached(webview, child_webview_id) => {
                write!(
                    formatter,
                    "managed child webview {child_webview_id} for webview {} already has an attached surface",
                    webview.raw()
                )
            }
            Self::ManagedChildSurfaceNotAttached(webview, child_webview_id) => {
                write!(
                    formatter,
                    "managed child webview {child_webview_id} for webview {} does not have an attached surface",
                    webview.raw()
                )
            }
            Self::InvalidUrl(error) => write!(formatter, "{error}"),
            Self::Host(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for RuntimeError {}

impl From<NavigationError> for RuntimeError {
    fn from(error: NavigationError) -> Self {
        Self::InvalidUrl(error)
    }
}

impl From<HostError> for RuntimeError {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}

pub trait Host {
    fn create_webview(
        &mut self,
        session: SessionHandle,
        webview: WebViewHandle,
    ) -> Result<(), HostError>;

    fn attach_surface(
        &mut self,
        _webview: WebViewHandle,
        _surface: &HostSurface,
        _size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn attach_surface_with_viewport(
        &mut self,
        webview: WebViewHandle,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        let mut events = self.attach_surface(webview, surface, viewport.size)?;
        events.extend(self.set_surface_scale_factor(webview, viewport.scale_factor)?);
        Ok(events)
    }

    fn resize_surface(
        &mut self,
        _webview: WebViewHandle,
        _size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn set_surface_scale_factor(
        &mut self,
        _webview: WebViewHandle,
        _scale_factor: f32,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn update_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        viewport: SurfaceViewport,
        previous_viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        let mut events = Vec::new();
        if previous_viewport.size != viewport.size {
            events.extend(self.resize_surface(webview, viewport.size)?);
        }
        if previous_viewport.scale_factor != viewport.scale_factor {
            events.extend(self.set_surface_scale_factor(webview, viewport.scale_factor)?);
        }
        Ok(events)
    }

    fn detach_surface(&mut self, _webview: WebViewHandle) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn set_popup_policy(
        &mut self,
        _webview: WebViewHandle,
        _popup_policy: PopupRequestPolicy,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn attach_managed_child_surface(
        &mut self,
        _webview: WebViewHandle,
        _child_webview_id: &str,
        _surface: &HostSurface,
        _viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn resize_managed_child_surface(
        &mut self,
        _webview: WebViewHandle,
        _child_webview_id: &str,
        _size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn update_managed_child_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        viewport: SurfaceViewport,
        previous_viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        let mut events = Vec::new();
        if previous_viewport.size != viewport.size {
            events.extend(self.resize_managed_child_surface(
                webview,
                child_webview_id,
                viewport.size,
            )?);
        }
        Ok(events)
    }

    fn detach_managed_child_surface(
        &mut self,
        _webview: WebViewHandle,
        _child_webview_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn perform_managed_child_updates(
        &mut self,
        _webview: WebViewHandle,
        _child_webview_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn dispatch_webview_command(
        &mut self,
        _webview: WebViewHandle,
        _command: &WebViewCommand,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn resolve_navigation_request(
        &mut self,
        _webview: WebViewHandle,
        navigation_id: &str,
        _allow: bool,
    ) -> Result<Vec<HostEvent>, HostError> {
        Err(HostError::new(format!(
            "navigation request {navigation_id} cannot be resolved by this host"
        )))
    }

    /// Forward a retained Servo simple-dialog completion to this host.
    ///
    /// Provenance is `WebViewDelegate::show_embedder_control` with
    /// `EmbedderControl::SimpleDialog`. Completion uses the retained dialog's
    /// confirm/dismiss operations and optional prompt `set_current_value`; this
    /// method adds no capability or policy beyond that existing Servo path.
    fn resolve_simple_dialog(
        &mut self,
        _webview: WebViewHandle,
        dialog_id: &str,
        _confirmed: bool,
        _prompt_value: Option<&str>,
    ) -> Result<Vec<HostEvent>, HostError> {
        Err(HostError::new(format!(
            "simple dialog {dialog_id} cannot be resolved by this host"
        )))
    }

    /// Forward a retained Servo context-menu selection to this host.
    ///
    /// Provenance is `WebViewDelegate::show_embedder_control` with
    /// `EmbedderControl::ContextMenu`, completed through its retained `select`
    /// operation. This method adds no capability or policy beyond that existing
    /// Servo path.
    fn resolve_context_menu(
        &mut self,
        _webview: WebViewHandle,
        context_menu_id: &str,
        _action: ContextMenuAction,
    ) -> Result<Vec<HostEvent>, HostError> {
        Err(HostError::new(format!(
            "context menu {context_menu_id} cannot be resolved by this host"
        )))
    }

    /// Forward dismissal of a retained Servo context menu to this host.
    ///
    /// Provenance is `WebViewDelegate::show_embedder_control` with
    /// `EmbedderControl::ContextMenu`, completed through its retained `dismiss`
    /// operation. This method adds no capability or policy beyond that existing
    /// Servo path.
    fn dismiss_context_menu(
        &mut self,
        _webview: WebViewHandle,
        context_menu_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        Err(HostError::new(format!(
            "context menu {context_menu_id} cannot be dismissed by this host"
        )))
    }

    /// Forward JavaScript evaluation through Servo's existing webview path.
    ///
    /// Provenance is `WebView::evaluate_javascript` and its completion callback.
    /// Immediate host dispatch failure returns [`HostError`]; accepted evaluation
    /// success or script/serialization failure completes asynchronously as
    /// [`HostEvent::JavaScriptEvaluationResult`] with the caller's identifier.
    /// This method adds no capability or policy beyond that existing Servo path.
    fn evaluate_javascript(
        &mut self,
        _webview: WebViewHandle,
        evaluation_id: &str,
        _script: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        Err(HostError::new(format!(
            "JavaScript evaluation {evaluation_id} is not supported by this host"
        )))
    }

    fn dispatch_input_event(
        &mut self,
        _webview: WebViewHandle,
        _event: &HostInputEvent,
    ) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn synthesize_webview_command_events(&self) -> bool {
        true
    }

    fn perform_updates(&mut self, _webview: WebViewHandle) -> Result<Vec<HostEvent>, HostError> {
        Ok(Vec::new())
    }

    fn observe_webview_events(
        &mut self,
        webview: WebViewHandle,
        events: &[HostEvent],
    ) -> Result<(), HostError>;

    fn observe_managed_child_webview_events(
        &mut self,
        webview: WebViewHandle,
        _child_webview_id: &str,
        events: &[HostEvent],
    ) -> Result<(), HostError> {
        self.observe_webview_events(webview, events)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AttachedSurface {
    surface: HostSurface,
    viewport: SurfaceViewport,
}

#[derive(Debug, Default)]
struct WebViewState {
    adapter: ServoAdapterState,
    surface: Option<AttachedSurface>,
    managed_child_surfaces: HashMap<String, AttachedSurface>,
    history: Vec<String>,
    history_index: Option<usize>,
    is_focused: bool,
}

impl WebViewState {
    fn current_history_url(&self) -> Option<&str> {
        self.history_index
            .and_then(|index| self.history.get(index))
            .map(String::as_str)
    }

    fn push_history_url(&mut self, url: String) {
        if let Some(index) = self.history_index {
            self.history.truncate(index + 1);
        }

        self.history.push(url);
        self.history_index = Some(self.history.len() - 1);
    }

    fn notify_history_changed(&mut self) {
        let Some(current) = self.history_index else {
            return;
        };

        self.adapter.notify_history_changed(
            self.history.clone(),
            current,
            current > 0,
            current + 1 < self.history.len(),
        );
    }
}

#[derive(Debug)]
pub struct Runtime<H> {
    host: H,
    next_session_id: u64,
    next_webview_id: u64,
    sessions: HashSet<SessionHandle>,
    webviews: HashMap<WebViewHandle, WebViewState>,
    events: VecDeque<ServokitEvent>,
}

impl<H: Host> Runtime<H> {
    pub fn new(host: H) -> Self {
        Self {
            host,
            next_session_id: 0,
            next_webview_id: 0,
            sessions: HashSet::new(),
            webviews: HashMap::new(),
            events: VecDeque::new(),
        }
    }

    pub fn create_session(&mut self) -> SessionHandle {
        self.next_session_id = self.next_session_id.wrapping_add(1);
        let session = SessionHandle(self.next_session_id);
        self.sessions.insert(session);
        session
    }

    pub fn create_webview(
        &mut self,
        session: SessionHandle,
    ) -> Result<WebViewHandle, RuntimeError> {
        if !self.sessions.contains(&session) {
            return Err(RuntimeError::UnknownSession(session));
        }

        self.next_webview_id = self.next_webview_id.wrapping_add(1);
        let webview = WebViewHandle(self.next_webview_id);
        self.host.create_webview(session, webview)?;
        self.webviews.insert(webview, WebViewState::default());
        Ok(webview)
    }

    pub fn attach_surface(
        &mut self,
        webview: WebViewHandle,
        surface: HostSurface,
        size: SurfaceSize,
    ) -> Result<(), RuntimeError> {
        let state = self.webview_state(webview)?;
        if state.surface.is_some() {
            return Err(RuntimeError::SurfaceAlreadyAttached(webview));
        }

        let host_events = self.host.attach_surface(webview, &surface, size)?;
        let state = self
            .webviews
            .get_mut(&webview)
            .expect("webview presence was checked");
        state.surface = Some(AttachedSurface {
            surface,
            viewport: SurfaceViewport::new(size, 1.0),
        });
        let mut events = vec![HostEvent::SurfaceAttached { size }];
        events.extend(host_events);
        self.record_events(webview, events)
    }

    pub fn resize_surface(
        &mut self,
        webview: WebViewHandle,
        size: SurfaceSize,
    ) -> Result<(), RuntimeError> {
        let state = self.webview_state(webview)?;
        if state.surface.is_none() {
            return Err(RuntimeError::SurfaceNotAttached(webview));
        }

        let host_events = self.host.resize_surface(webview, size)?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .surface
            .as_mut()
            .expect("surface presence was checked")
            .viewport
            .size = size;
        let mut events = vec![HostEvent::SurfaceResized { size }];
        events.extend(host_events);
        self.record_events(webview, events)
    }

    pub fn set_surface_scale_factor(
        &mut self,
        webview: WebViewHandle,
        scale_factor: f32,
    ) -> Result<(), RuntimeError> {
        let state = self.webview_state(webview)?;
        if state.surface.is_none() {
            return Err(RuntimeError::SurfaceNotAttached(webview));
        }

        let events = self.host.set_surface_scale_factor(webview, scale_factor)?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .surface
            .as_mut()
            .expect("surface presence was checked")
            .viewport
            .scale_factor = scale_factor;
        self.record_events(webview, events)
    }

    pub fn attach_surface_with_viewport(
        &mut self,
        webview: WebViewHandle,
        surface: HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<(), RuntimeError> {
        let state = self.webview_state(webview)?;
        if state.surface.is_some() {
            return Err(RuntimeError::SurfaceAlreadyAttached(webview));
        }

        let host_events = self
            .host
            .attach_surface_with_viewport(webview, &surface, viewport)?;
        let state = self
            .webviews
            .get_mut(&webview)
            .expect("webview presence was checked");
        state.surface = Some(AttachedSurface { surface, viewport });
        let mut events = vec![HostEvent::SurfaceAttached {
            size: viewport.size,
        }];
        events.extend(host_events);
        self.record_events(webview, events)
    }

    pub fn update_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        viewport: SurfaceViewport,
    ) -> Result<(), RuntimeError> {
        let state = self.webview_state(webview)?;
        let Some(attached_surface) = &state.surface else {
            return Err(RuntimeError::SurfaceNotAttached(webview));
        };
        let previous_viewport = attached_surface.viewport;

        let host_events =
            self.host
                .update_surface_viewport(webview, viewport, previous_viewport)?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .surface
            .as_mut()
            .expect("surface presence was checked")
            .viewport = viewport;
        let mut events = Vec::new();
        if previous_viewport.size != viewport.size {
            events.push(HostEvent::SurfaceResized {
                size: viewport.size,
            });
        }
        events.extend(host_events);
        self.record_events(webview, events)
    }

    pub fn detach_surface(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        let state = self.webview_state(webview)?;
        if state.surface.is_none() {
            return Err(RuntimeError::SurfaceNotAttached(webview));
        }

        let host_events = self.host.detach_surface(webview)?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .surface = None;
        let mut events = vec![HostEvent::SurfaceDetached];
        events.extend(host_events);
        self.record_events(webview, events)
    }

    pub fn set_popup_policy(
        &mut self,
        webview: WebViewHandle,
        popup_policy: PopupRequestPolicy,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let events = self.host.set_popup_policy(webview, popup_policy)?;
        self.record_events(webview, events)
    }

    pub fn attach_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: impl Into<String>,
        surface: HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<(), RuntimeError> {
        let child_webview_id = child_webview_id.into();
        let state = self.webview_state(webview)?;
        if state.managed_child_surfaces.contains_key(&child_webview_id) {
            return Err(RuntimeError::ManagedChildSurfaceAlreadyAttached(
                webview,
                child_webview_id,
            ));
        }

        let host_events = self.host.attach_managed_child_surface(
            webview,
            &child_webview_id,
            &surface,
            viewport,
        )?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .managed_child_surfaces
            .insert(
                child_webview_id.clone(),
                AttachedSurface { surface, viewport },
            );
        let mut events = vec![HostEvent::SurfaceAttached {
            size: viewport.size,
        }];
        events.extend(host_events);
        self.record_managed_child_events(webview, child_webview_id, events)
    }

    pub fn resize_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: impl AsRef<str>,
        size: SurfaceSize,
    ) -> Result<(), RuntimeError> {
        let child_webview_id = child_webview_id.as_ref();
        let state = self.webview_state(webview)?;
        if !state.managed_child_surfaces.contains_key(child_webview_id) {
            return Err(RuntimeError::ManagedChildSurfaceNotAttached(
                webview,
                child_webview_id.to_owned(),
            ));
        }

        let host_events =
            self.host
                .resize_managed_child_surface(webview, child_webview_id, size)?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .managed_child_surfaces
            .get_mut(child_webview_id)
            .expect("child surface presence was checked")
            .viewport
            .size = size;
        let mut events = vec![HostEvent::SurfaceResized { size }];
        events.extend(host_events);
        self.record_managed_child_events(webview, child_webview_id.to_owned(), events)
    }

    pub fn update_managed_child_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: impl AsRef<str>,
        viewport: SurfaceViewport,
    ) -> Result<(), RuntimeError> {
        let child_webview_id = child_webview_id.as_ref();
        let state = self.webview_state(webview)?;
        let Some(attached_surface) = state.managed_child_surfaces.get(child_webview_id) else {
            return Err(RuntimeError::ManagedChildSurfaceNotAttached(
                webview,
                child_webview_id.to_owned(),
            ));
        };
        let previous_viewport = attached_surface.viewport;

        let host_events = self.host.update_managed_child_surface_viewport(
            webview,
            child_webview_id,
            viewport,
            previous_viewport,
        )?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .managed_child_surfaces
            .get_mut(child_webview_id)
            .expect("child surface presence was checked")
            .viewport = viewport;
        let mut events = Vec::new();
        if previous_viewport.size != viewport.size {
            events.push(HostEvent::SurfaceResized {
                size: viewport.size,
            });
        }
        events.extend(host_events);
        self.record_managed_child_events(webview, child_webview_id.to_owned(), events)
    }

    pub fn detach_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: impl AsRef<str>,
    ) -> Result<(), RuntimeError> {
        let child_webview_id = child_webview_id.as_ref();
        let state = self.webview_state(webview)?;
        if !state.managed_child_surfaces.contains_key(child_webview_id) {
            return Err(RuntimeError::ManagedChildSurfaceNotAttached(
                webview,
                child_webview_id.to_owned(),
            ));
        }

        let host_events = self
            .host
            .detach_managed_child_surface(webview, child_webview_id)?;
        self.webviews
            .get_mut(&webview)
            .expect("webview presence was checked")
            .managed_child_surfaces
            .remove(child_webview_id);
        let mut events = vec![HostEvent::SurfaceDetached];
        events.extend(host_events);
        self.record_managed_child_events(webview, child_webview_id.to_owned(), events)
    }

    pub fn perform_managed_child_updates(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: impl AsRef<str>,
    ) -> Result<(), RuntimeError> {
        let child_webview_id = child_webview_id.as_ref();
        let state = self.webview_state(webview)?;
        if !state.managed_child_surfaces.contains_key(child_webview_id) {
            return Err(RuntimeError::ManagedChildSurfaceNotAttached(
                webview,
                child_webview_id.to_owned(),
            ));
        }

        let events = self
            .host
            .perform_managed_child_updates(webview, child_webview_id)?;
        self.record_managed_child_events(webview, child_webview_id.to_owned(), events)
    }

    pub fn load_url(
        &mut self,
        webview: WebViewHandle,
        url: impl AsRef<str>,
    ) -> Result<(), RuntimeError> {
        let request = NavigationRequest::new(url.as_ref())?;
        self.dispatch_webview_command(webview, WebViewCommand::LoadUrl(request))
    }

    pub fn reload(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        self.dispatch_webview_command(webview, WebViewCommand::Reload)
    }

    pub fn go_back(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        self.dispatch_webview_command(webview, WebViewCommand::GoBack)
    }

    pub fn go_forward(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        self.dispatch_webview_command(webview, WebViewCommand::GoForward)
    }

    pub fn focus(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        self.dispatch_webview_command(webview, WebViewCommand::Focus)
    }

    pub fn blur(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        self.dispatch_webview_command(webview, WebViewCommand::Blur)
    }

    pub fn dispatch_webview_command(
        &mut self,
        webview: WebViewHandle,
        command: WebViewCommand,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let synthesize_events = self.host.synthesize_webview_command_events();
        let mut events = self.host.dispatch_webview_command(webview, &command)?;

        if synthesize_events {
            let state = self
                .webviews
                .get_mut(&webview)
                .expect("webview presence was checked");
            events.extend(dispatch_embedder_command(state, command));
        }
        self.record_events(webview, events)
    }

    /// Resolve a Servo `WebViewDelegate::request_navigation` policy request.
    ///
    /// The `navigation_id` comes from ServoKit's `NavigationRequested` host event.
    /// `allow=true` calls Servo's retained request `allow`; `allow=false` calls `deny`.
    pub fn resolve_navigation_request(
        &mut self,
        webview: WebViewHandle,
        navigation_id: impl AsRef<str>,
        allow: bool,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let navigation_id = navigation_id.as_ref();
        if navigation_id.trim().is_empty() {
            return Err(RuntimeError::Host(HostError::new(
                "navigation request id must not be empty",
            )));
        }

        let events = self
            .host
            .resolve_navigation_request(webview, navigation_id, allow)?;
        self.record_events(webview, events)
    }

    /// Resolve a retained Servo simple-dialog request through [`Host`].
    ///
    /// Provenance is `WebViewDelegate::show_embedder_control` with
    /// `EmbedderControl::SimpleDialog`. Completion uses the retained dialog's
    /// confirm/dismiss operations and optional prompt `set_current_value`; this
    /// method adds no capability or policy beyond that existing Servo path.
    pub fn resolve_simple_dialog(
        &mut self,
        webview: WebViewHandle,
        dialog_id: impl AsRef<str>,
        confirmed: bool,
        prompt_value: Option<&str>,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let dialog_id = dialog_id.as_ref();
        if dialog_id.trim().is_empty() {
            return Err(RuntimeError::Host(HostError::new(
                "simple dialog id must not be empty",
            )));
        }

        let events =
            self.host
                .resolve_simple_dialog(webview, dialog_id, confirmed, prompt_value)?;
        self.record_events(webview, events)
    }

    /// Resolve a retained Servo context-menu request through [`Host`].
    ///
    /// Provenance is `WebViewDelegate::show_embedder_control` with
    /// `EmbedderControl::ContextMenu`, completed through its retained `select`
    /// operation. This method adds no capability or policy beyond that existing
    /// Servo path.
    pub fn resolve_context_menu(
        &mut self,
        webview: WebViewHandle,
        context_menu_id: impl AsRef<str>,
        action: ContextMenuAction,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let context_menu_id = context_menu_id.as_ref();
        if context_menu_id.trim().is_empty() {
            return Err(RuntimeError::Host(HostError::new(
                "context menu id must not be empty",
            )));
        }

        let events = self
            .host
            .resolve_context_menu(webview, context_menu_id, action)?;
        self.record_events(webview, events)
    }

    /// Dismiss a retained Servo context-menu request through [`Host`].
    ///
    /// Provenance is `WebViewDelegate::show_embedder_control` with
    /// `EmbedderControl::ContextMenu`, completed through its retained `dismiss`
    /// operation. This method adds no capability or policy beyond that existing
    /// Servo path.
    pub fn dismiss_context_menu(
        &mut self,
        webview: WebViewHandle,
        context_menu_id: impl AsRef<str>,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let context_menu_id = context_menu_id.as_ref();
        if context_menu_id.trim().is_empty() {
            return Err(RuntimeError::Host(HostError::new(
                "context menu id must not be empty",
            )));
        }

        let events = self.host.dismiss_context_menu(webview, context_menu_id)?;
        self.record_events(webview, events)
    }

    /// Evaluate JavaScript through Servo's `WebView::evaluate_javascript` path.
    ///
    /// The asynchronous result is emitted as `HostEvent::JavaScriptEvaluationResult` with the
    /// caller-provided evaluation identifier.
    pub fn evaluate_javascript(
        &mut self,
        webview: WebViewHandle,
        evaluation_id: impl AsRef<str>,
        script: impl AsRef<str>,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let evaluation_id = evaluation_id.as_ref();
        if evaluation_id.trim().is_empty() {
            return Err(RuntimeError::Host(HostError::new(
                "JavaScript evaluation id must not be empty",
            )));
        }
        let events = self
            .host
            .evaluate_javascript(webview, evaluation_id, script.as_ref())?;
        self.record_events(webview, events)
    }

    pub fn dispatch_input_event(
        &mut self,
        webview: WebViewHandle,
        event: HostInputEvent,
    ) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let events = self.host.dispatch_input_event(webview, &event)?;
        self.record_events(webview, events)
    }

    pub fn perform_updates(&mut self, webview: WebViewHandle) -> Result<(), RuntimeError> {
        self.webview_state(webview)?;
        let events = self.host.perform_updates(webview)?;
        self.record_events(webview, events)
    }

    pub fn drain_events(&mut self) -> Vec<ServokitEvent> {
        self.events.drain(..).collect()
    }

    fn webview_state(&self, webview: WebViewHandle) -> Result<&WebViewState, RuntimeError> {
        self.webviews
            .get(&webview)
            .ok_or(RuntimeError::UnknownWebView(webview))
    }

    fn record_events(
        &mut self,
        webview: WebViewHandle,
        events: Vec<HostEvent>,
    ) -> Result<(), RuntimeError> {
        self.record_events_with_target(webview, None, events)
    }

    fn record_managed_child_events(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: String,
        events: Vec<HostEvent>,
    ) -> Result<(), RuntimeError> {
        self.record_events_with_target(webview, Some(child_webview_id), events)
    }

    fn record_events_with_target(
        &mut self,
        webview: WebViewHandle,
        managed_child_webview_id: Option<String>,
        events: Vec<HostEvent>,
    ) -> Result<(), RuntimeError> {
        if events.is_empty() {
            return Ok(());
        }

        if let Some(child_webview_id) = managed_child_webview_id.as_deref() {
            self.host
                .observe_managed_child_webview_events(webview, child_webview_id, &events)?;
        } else {
            self.host.observe_webview_events(webview, &events)?;
        }
        self.events
            .extend(events.into_iter().map(|event| ServokitEvent {
                webview,
                managed_child_webview_id: managed_child_webview_id.clone(),
                event,
            }));
        Ok(())
    }
}

fn dispatch_embedder_command(state: &mut WebViewState, command: WebViewCommand) -> Vec<HostEvent> {
    match command {
        WebViewCommand::LoadUrl(request) => {
            let url = request.url;
            state.push_history_url(url.clone());
            emit_navigation_events(state, &url, true);
        }
        WebViewCommand::Reload => {
            if state.current_history_url().is_some() {
                state
                    .adapter
                    .notify_load_status_changed(LoadStatusKind::Started);
                state
                    .adapter
                    .notify_load_status_changed(LoadStatusKind::Complete);
            }
        }
        WebViewCommand::GoBack => {
            if let Some(current) = state.history_index.filter(|current| *current > 0) {
                let target = current - 1;
                state.history_index = Some(target);
                let url = state.history[target].clone();
                emit_navigation_events(state, &url, true);
            }
        }
        WebViewCommand::GoForward => {
            if let Some(current) = state
                .history_index
                .filter(|current| current + 1 < state.history.len())
            {
                let target = current + 1;
                state.history_index = Some(target);
                let url = state.history[target].clone();
                emit_navigation_events(state, &url, true);
            }
        }
        WebViewCommand::Focus => {
            state.is_focused = true;
            state.adapter.notify_focus_changed(true);
        }
        WebViewCommand::Blur => {
            state.is_focused = false;
            state.adapter.notify_focus_changed(false);
        }
    }

    state.adapter.drain_events()
}

fn emit_navigation_events(state: &mut WebViewState, url: &str, include_history: bool) {
    state
        .adapter
        .notify_load_status_changed(LoadStatusKind::Started);
    state.adapter.notify_url_changed(url);
    if include_history {
        state.notify_history_changed();
    }
    state
        .adapter
        .notify_load_status_changed(LoadStatusKind::Complete);
}

#[derive(Debug, Default)]
pub struct PlaceholderHost {
    created_webviews: HashMap<WebViewHandle, SessionHandle>,
    attached_surfaces: HashMap<WebViewHandle, AttachedSurface>,
    managed_child_surfaces: HashMap<(WebViewHandle, String), AttachedSurface>,
    popup_policies: HashMap<WebViewHandle, PopupRequestPolicy>,
    calls: Vec<HostCall>,
    observed_events: Vec<ServokitEvent>,
    queued_update_events: VecDeque<HostEvent>,
    queued_managed_child_update_events: HashMap<(WebViewHandle, String), VecDeque<HostEvent>>,
}

pub type MockHost = PlaceholderHost;

impl PlaceholderHost {
    pub fn with_update_events(events: impl IntoIterator<Item = HostEvent>) -> Self {
        Self {
            queued_update_events: events.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn calls(&self) -> &[HostCall] {
        &self.calls
    }

    pub fn observed_events(&self) -> &[ServokitEvent] {
        &self.observed_events
    }

    pub fn queue_update_event(&mut self, event: HostEvent) {
        self.queued_update_events.push_back(event);
    }

    pub fn queue_managed_child_update_event(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: impl Into<String>,
        event: HostEvent,
    ) {
        self.queued_managed_child_update_events
            .entry((webview, child_webview_id.into()))
            .or_default()
            .push_back(event);
    }

    fn ensure_created_webview(&self, webview: WebViewHandle) -> Result<(), HostError> {
        self.created_webviews
            .contains_key(&webview)
            .then_some(())
            .ok_or_else(|| HostError::new(format!("webview {} is not registered", webview.raw())))
    }

    fn ensure_managed_child_surface(
        &self,
        webview: WebViewHandle,
        child_webview_id: &str,
    ) -> Result<(), HostError> {
        self.ensure_created_webview(webview)?;
        self.managed_child_surfaces
            .contains_key(&(webview, child_webview_id.to_owned()))
            .then_some(())
            .ok_or_else(|| {
                HostError::new(format!(
                    "managed child webview {child_webview_id} for webview {} does not have an attached surface",
                    webview.raw()
                ))
            })
    }
}

impl Host for PlaceholderHost {
    fn create_webview(
        &mut self,
        session: SessionHandle,
        webview: WebViewHandle,
    ) -> Result<(), HostError> {
        if self.created_webviews.contains_key(&webview) {
            return Err(HostError::new(format!(
                "webview {} is already registered",
                webview.raw()
            )));
        }

        self.created_webviews.insert(webview, session);
        self.calls
            .push(HostCall::CreateWebView { session, webview });
        Ok(())
    }

    fn attach_surface(
        &mut self,
        webview: WebViewHandle,
        surface: &HostSurface,
        size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        if self.attached_surfaces.contains_key(&webview) {
            return Err(HostError::new(format!(
                "webview {} already has an attached surface",
                webview.raw()
            )));
        }

        self.attached_surfaces.insert(
            webview,
            AttachedSurface {
                surface: surface.clone(),
                viewport: SurfaceViewport::new(size, 1.0),
            },
        );
        self.calls.push(HostCall::AttachSurface {
            webview,
            surface: surface.clone(),
            size,
        });
        Ok(Vec::new())
    }

    fn attach_surface_with_viewport(
        &mut self,
        webview: WebViewHandle,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        if self.attached_surfaces.contains_key(&webview) {
            return Err(HostError::new(format!(
                "webview {} already has an attached surface",
                webview.raw()
            )));
        }

        self.attached_surfaces.insert(
            webview,
            AttachedSurface {
                surface: surface.clone(),
                viewport,
            },
        );
        self.calls.push(HostCall::AttachSurfaceWithViewport {
            webview,
            surface: surface.clone(),
            viewport,
        });
        Ok(Vec::new())
    }

    fn resize_surface(
        &mut self,
        webview: WebViewHandle,
        size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        let Some(surface) = self.attached_surfaces.get_mut(&webview) else {
            return Err(HostError::new(format!(
                "webview {} does not have an attached surface",
                webview.raw()
            )));
        };

        surface.viewport.size = size;
        self.calls.push(HostCall::ResizeSurface { webview, size });
        Ok(Vec::new())
    }

    fn set_surface_scale_factor(
        &mut self,
        webview: WebViewHandle,
        scale_factor: f32,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        let Some(surface) = self.attached_surfaces.get_mut(&webview) else {
            return Err(HostError::new(format!(
                "webview {} does not have an attached surface",
                webview.raw()
            )));
        };

        surface.viewport.scale_factor = scale_factor;
        self.calls.push(HostCall::SetSurfaceScaleFactor {
            webview,
            scale_factor,
        });
        Ok(Vec::new())
    }

    fn update_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        viewport: SurfaceViewport,
        _previous_viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        let Some(surface) = self.attached_surfaces.get_mut(&webview) else {
            return Err(HostError::new(format!(
                "webview {} does not have an attached surface",
                webview.raw()
            )));
        };

        surface.viewport = viewport;
        self.calls
            .push(HostCall::UpdateSurfaceViewport { webview, viewport });
        Ok(Vec::new())
    }

    fn detach_surface(&mut self, webview: WebViewHandle) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        if self.attached_surfaces.remove(&webview).is_none() {
            return Err(HostError::new(format!(
                "webview {} does not have an attached surface",
                webview.raw()
            )));
        }

        self.calls.push(HostCall::DetachSurface { webview });
        Ok(Vec::new())
    }

    fn set_popup_policy(
        &mut self,
        webview: WebViewHandle,
        popup_policy: PopupRequestPolicy,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        self.popup_policies.insert(webview, popup_policy);
        self.calls.push(HostCall::SetPopupPolicy {
            webview,
            popup_policy,
        });
        Ok(Vec::new())
    }

    fn attach_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        surface: &HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        let key = (webview, child_webview_id.to_owned());
        if self.managed_child_surfaces.contains_key(&key) {
            return Err(HostError::new(format!(
                "managed child webview {child_webview_id} for webview {} already has an attached surface",
                webview.raw()
            )));
        }

        self.managed_child_surfaces.insert(
            key,
            AttachedSurface {
                surface: surface.clone(),
                viewport,
            },
        );
        self.calls.push(HostCall::AttachManagedChildSurface {
            webview,
            child_webview_id: child_webview_id.to_owned(),
            surface: surface.clone(),
            viewport,
        });
        Ok(Vec::new())
    }

    fn resize_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        size: SurfaceSize,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_managed_child_surface(webview, child_webview_id)?;
        self.managed_child_surfaces
            .get_mut(&(webview, child_webview_id.to_owned()))
            .expect("managed child surface presence was checked")
            .viewport
            .size = size;
        self.calls.push(HostCall::ResizeManagedChildSurface {
            webview,
            child_webview_id: child_webview_id.to_owned(),
            size,
        });
        Ok(Vec::new())
    }

    fn update_managed_child_surface_viewport(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        viewport: SurfaceViewport,
        _previous_viewport: SurfaceViewport,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_managed_child_surface(webview, child_webview_id)?;
        self.managed_child_surfaces
            .get_mut(&(webview, child_webview_id.to_owned()))
            .expect("managed child surface presence was checked")
            .viewport = viewport;
        self.calls
            .push(HostCall::UpdateManagedChildSurfaceViewport {
                webview,
                child_webview_id: child_webview_id.to_owned(),
                viewport,
            });
        Ok(Vec::new())
    }

    fn detach_managed_child_surface(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_managed_child_surface(webview, child_webview_id)?;
        self.managed_child_surfaces
            .remove(&(webview, child_webview_id.to_owned()));
        self.calls.push(HostCall::DetachManagedChildSurface {
            webview,
            child_webview_id: child_webview_id.to_owned(),
        });
        Ok(Vec::new())
    }

    fn perform_managed_child_updates(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_managed_child_surface(webview, child_webview_id)?;
        self.calls.push(HostCall::PerformManagedChildUpdates {
            webview,
            child_webview_id: child_webview_id.to_owned(),
        });
        Ok(self
            .queued_managed_child_update_events
            .entry((webview, child_webview_id.to_owned()))
            .or_default()
            .drain(..)
            .collect())
    }

    fn dispatch_webview_command(
        &mut self,
        webview: WebViewHandle,
        command: &WebViewCommand,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        self.calls.push(HostCall::DispatchWebViewCommand {
            webview,
            command: command.clone(),
        });
        Ok(Vec::new())
    }

    fn perform_updates(&mut self, webview: WebViewHandle) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        self.calls.push(HostCall::PerformUpdates { webview });
        Ok(self.queued_update_events.drain(..).collect())
    }

    fn resolve_navigation_request(
        &mut self,
        webview: WebViewHandle,
        navigation_id: &str,
        allow: bool,
    ) -> Result<Vec<HostEvent>, HostError> {
        self.ensure_created_webview(webview)?;
        self.calls.push(HostCall::ResolveNavigationRequest {
            webview,
            navigation_id: navigation_id.to_owned(),
            allow,
        });
        Ok(Vec::new())
    }

    fn observe_webview_events(
        &mut self,
        webview: WebViewHandle,
        events: &[HostEvent],
    ) -> Result<(), HostError> {
        self.ensure_created_webview(webview)?;
        self.observed_events
            .extend(events.iter().cloned().map(|event| ServokitEvent {
                webview,
                managed_child_webview_id: None,
                event,
            }));
        Ok(())
    }

    fn observe_managed_child_webview_events(
        &mut self,
        webview: WebViewHandle,
        child_webview_id: &str,
        events: &[HostEvent],
    ) -> Result<(), HostError> {
        self.ensure_created_webview(webview)?;
        self.observed_events
            .extend(events.iter().cloned().map(|event| ServokitEvent {
                webview,
                managed_child_webview_id: Some(child_webview_id.to_owned()),
                event,
            }));
        Ok(())
    }
}

#[cfg(test)]
impl<H> Runtime<H> {
    fn host(&self) -> &H {
        &self.host
    }

    fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        KeyboardInputEvent, KeyboardInputKey, KeyboardInputState, PointerInputEvent,
        PopupRequestPolicy,
    };

    fn event(webview: WebViewHandle, event: HostEvent) -> ServokitEvent {
        ServokitEvent {
            webview,
            managed_child_webview_id: None,
            event,
        }
    }

    fn child_event(
        webview: WebViewHandle,
        child_webview_id: impl Into<String>,
        event: HostEvent,
    ) -> ServokitEvent {
        ServokitEvent {
            webview,
            managed_child_webview_id: Some(child_webview_id.into()),
            event,
        }
    }

    fn load_events(
        webview: WebViewHandle,
        url: &str,
        history: Vec<String>,
        current: usize,
    ) -> Vec<ServokitEvent> {
        vec![
            event(
                webview,
                HostEvent::LoadStatusChanged {
                    status: LoadStatusKind::Started,
                },
            ),
            event(
                webview,
                HostEvent::UrlChanged {
                    url: url.to_owned(),
                },
            ),
            event(
                webview,
                HostEvent::HistoryChanged {
                    can_go_back: current > 0,
                    can_go_forward: current + 1 < history.len(),
                    entries: history,
                    current,
                },
            ),
            event(
                webview,
                HostEvent::LoadStatusChanged {
                    status: LoadStatusKind::Complete,
                },
            ),
        ]
    }

    #[derive(Default)]
    struct InputRecordingHost {
        created_webviews: HashSet<WebViewHandle>,
        inputs: Vec<(WebViewHandle, HostInputEvent)>,
        observed_events: Vec<ServokitEvent>,
    }

    impl Host for InputRecordingHost {
        fn create_webview(
            &mut self,
            _session: SessionHandle,
            webview: WebViewHandle,
        ) -> Result<(), HostError> {
            self.created_webviews.insert(webview);
            Ok(())
        }

        fn dispatch_input_event(
            &mut self,
            webview: WebViewHandle,
            input: &HostInputEvent,
        ) -> Result<Vec<HostEvent>, HostError> {
            if !self.created_webviews.contains(&webview) {
                return Err(HostError::new(format!(
                    "webview {} is not registered",
                    webview.raw()
                )));
            }

            self.inputs.push((webview, input.clone()));
            match input {
                HostInputEvent::Focus { is_focused } => Ok(vec![HostEvent::FocusChanged {
                    is_focused: *is_focused,
                }]),
                _ => Ok(Vec::new()),
            }
        }

        fn observe_webview_events(
            &mut self,
            webview: WebViewHandle,
            events: &[HostEvent],
        ) -> Result<(), HostError> {
            self.observed_events
                .extend(events.iter().cloned().map(|event| ServokitEvent {
                    webview,
                    managed_child_webview_id: None,
                    event,
                }));
            Ok(())
        }
    }

    #[test]
    fn native_caller_can_create_handles_load_url_and_receive_servo_events() {
        let mut runtime = Runtime::new(PlaceholderHost::default());

        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        runtime.load_url(webview, "example.com").unwrap();

        assert_eq!(session.raw(), 1);
        assert_eq!(webview.raw(), 1);

        let expected_events = load_events(
            webview,
            "https://example.com/",
            vec!["https://example.com/".to_owned()],
            0,
        );

        assert_eq!(runtime.host().observed_events(), expected_events.as_slice());
        assert_eq!(runtime.drain_events(), expected_events);
        assert!(runtime.drain_events().is_empty());
    }

    #[test]
    fn runtime_validates_session_webview_url_and_surface_state_before_host_dispatch() {
        let mut runtime = Runtime::new(PlaceholderHost::default());
        let unknown_session = SessionHandle(99);

        assert_eq!(
            runtime.create_webview(unknown_session),
            Err(RuntimeError::UnknownSession(unknown_session))
        );
        assert!(runtime.host().calls().is_empty());
        assert!(runtime.host().observed_events().is_empty());

        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        let calls_after_create = runtime.host().calls().len();

        assert!(matches!(
            runtime.load_url(webview, "   "),
            Err(RuntimeError::InvalidUrl(NavigationError::Empty))
        ));
        assert!(matches!(
            runtime.load_url(webview, "http://[::1"),
            Err(RuntimeError::InvalidUrl(NavigationError::Invalid(_)))
        ));
        assert_eq!(runtime.host().calls().len(), calls_after_create);

        let unknown_webview = WebViewHandle(42);
        assert_eq!(
            runtime.load_url(unknown_webview, "example.com"),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.dispatch_webview_command(unknown_webview, WebViewCommand::Reload),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.attach_surface(
                unknown_webview,
                HostSurface::new("missing-surface"),
                SurfaceSize::new(1, 1),
            ),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.resize_surface(unknown_webview, SurfaceSize::new(640, 480)),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.set_surface_scale_factor(unknown_webview, 2.0),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.update_surface_viewport(
                unknown_webview,
                SurfaceViewport::new(SurfaceSize::new(640, 480), 2.0),
            ),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.detach_surface(unknown_webview),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.perform_updates(unknown_webview),
            Err(RuntimeError::UnknownWebView(unknown_webview))
        );
        assert_eq!(
            runtime.resize_surface(webview, SurfaceSize::new(640, 480)),
            Err(RuntimeError::SurfaceNotAttached(webview))
        );
        assert_eq!(
            runtime.set_surface_scale_factor(webview, 2.0),
            Err(RuntimeError::SurfaceNotAttached(webview))
        );
        assert_eq!(
            runtime.update_surface_viewport(
                webview,
                SurfaceViewport::new(SurfaceSize::new(640, 480), 2.0),
            ),
            Err(RuntimeError::SurfaceNotAttached(webview))
        );
        assert_eq!(
            runtime.detach_surface(webview),
            Err(RuntimeError::SurfaceNotAttached(webview))
        );
        assert_eq!(runtime.host().calls().len(), calls_after_create);
        assert!(runtime.host().observed_events().is_empty());
        assert!(runtime.drain_events().is_empty());
    }

    #[test]
    fn runtime_dispatches_host_input_events_to_host_and_records_events() {
        let mut runtime = Runtime::new(InputRecordingHost::default());
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        let pointer = HostInputEvent::Pointer(PointerInputEvent::moved(12.0, 34.0));
        let keyboard = HostInputEvent::Keyboard(KeyboardInputEvent::new(
            KeyboardInputKey::Character("a".to_owned()),
            KeyboardInputState::Pressed,
        ));
        let focus = HostInputEvent::Focus { is_focused: true };

        runtime
            .dispatch_input_event(webview, pointer.clone())
            .unwrap();
        runtime
            .dispatch_input_event(webview, keyboard.clone())
            .unwrap();
        runtime
            .dispatch_input_event(webview, focus.clone())
            .unwrap();

        assert_eq!(
            runtime.host().inputs,
            vec![(webview, pointer), (webview, keyboard), (webview, focus)]
        );
        let expected_events = vec![event(webview, HostEvent::FocusChanged { is_focused: true })];
        assert_eq!(runtime.host().observed_events, expected_events);
        assert_eq!(runtime.drain_events(), expected_events);
    }

    #[test]
    fn runtime_resolves_navigation_policy_requests_through_host() {
        let mut runtime = Runtime::new(PlaceholderHost::default());
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();

        runtime
            .resolve_navigation_request(webview, "navigation-1", true)
            .unwrap();

        assert_eq!(
            runtime.host().calls(),
            &[
                HostCall::CreateWebView { session, webview },
                HostCall::ResolveNavigationRequest {
                    webview,
                    navigation_id: "navigation-1".to_owned(),
                    allow: true,
                },
            ]
        );
        assert!(runtime.drain_events().is_empty());
    }

    #[test]
    fn surface_lifecycle_crosses_the_host_boundary_and_emits_shared_events() {
        let mut runtime = Runtime::new(PlaceholderHost::default());
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        let surface = HostSurface::new("mock-surface");
        let initial_size = SurfaceSize::new(640, 480);
        let resized_size = SurfaceSize::new(800, 600);
        let reattached_size = SurfaceSize::new(1024, 768);
        let initial_viewport = SurfaceViewport::new(initial_size, 2.0);

        runtime
            .attach_surface_with_viewport(webview, surface.clone(), initial_viewport)
            .unwrap();
        assert_eq!(
            runtime.attach_surface(webview, surface.clone(), initial_size),
            Err(RuntimeError::SurfaceAlreadyAttached(webview))
        );
        runtime
            .update_surface_viewport(webview, SurfaceViewport::new(resized_size, 2.5))
            .unwrap();
        runtime.detach_surface(webview).unwrap();

        let calls_after_detach = runtime.host().calls().len();
        let observed_events_after_detach = runtime.host().observed_events().len();
        assert_eq!(
            runtime.resize_surface(webview, SurfaceSize::new(320, 240)),
            Err(RuntimeError::SurfaceNotAttached(webview))
        );
        assert_eq!(
            runtime.detach_surface(webview),
            Err(RuntimeError::SurfaceNotAttached(webview))
        );
        assert_eq!(runtime.host().calls().len(), calls_after_detach);
        assert_eq!(
            runtime.host().observed_events().len(),
            observed_events_after_detach
        );

        runtime
            .attach_surface(webview, surface.clone(), reattached_size)
            .unwrap();

        assert_eq!(
            runtime.host().calls(),
            &[
                HostCall::CreateWebView { session, webview },
                HostCall::AttachSurfaceWithViewport {
                    webview,
                    surface: surface.clone(),
                    viewport: initial_viewport,
                },
                HostCall::UpdateSurfaceViewport {
                    webview,
                    viewport: SurfaceViewport::new(resized_size, 2.5),
                },
                HostCall::DetachSurface { webview },
                HostCall::AttachSurface {
                    webview,
                    surface,
                    size: reattached_size,
                },
            ]
        );

        let expected_events = vec![
            event(webview, HostEvent::SurfaceAttached { size: initial_size }),
            event(webview, HostEvent::SurfaceResized { size: resized_size }),
            event(webview, HostEvent::SurfaceDetached),
            event(
                webview,
                HostEvent::SurfaceAttached {
                    size: reattached_size,
                },
            ),
        ];
        assert_eq!(runtime.host().observed_events(), expected_events.as_slice());
        assert_eq!(runtime.drain_events(), expected_events);
    }

    #[test]
    fn core_browser_commands_are_dispatched_and_emit_ordered_events() {
        let mut runtime = Runtime::new(PlaceholderHost::default());
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        let first = "https://servo.org/".to_owned();
        let second = "https://example.org/page".to_owned();

        runtime.load_url(webview, "servo.org").unwrap();
        runtime.load_url(webview, &second).unwrap();
        runtime.reload(webview).unwrap();
        runtime.go_back(webview).unwrap();
        runtime.go_forward(webview).unwrap();
        runtime.focus(webview).unwrap();
        runtime.blur(webview).unwrap();

        let command_calls: Vec<_> = runtime
            .host()
            .calls()
            .iter()
            .filter_map(|call| match call {
                HostCall::DispatchWebViewCommand { webview, command } => {
                    Some((*webview, command.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            command_calls,
            vec![
                (
                    webview,
                    WebViewCommand::LoadUrl(NavigationRequest::new("servo.org").unwrap()),
                ),
                (
                    webview,
                    WebViewCommand::LoadUrl(NavigationRequest::new(&second).unwrap()),
                ),
                (webview, WebViewCommand::Reload),
                (webview, WebViewCommand::GoBack),
                (webview, WebViewCommand::GoForward),
                (webview, WebViewCommand::Focus),
                (webview, WebViewCommand::Blur),
            ]
        );

        let mut expected_events = load_events(webview, &first, vec![first.clone()], 0);
        expected_events.extend(load_events(
            webview,
            &second,
            vec![first.clone(), second.clone()],
            1,
        ));
        expected_events.extend([
            event(
                webview,
                HostEvent::LoadStatusChanged {
                    status: LoadStatusKind::Started,
                },
            ),
            event(
                webview,
                HostEvent::LoadStatusChanged {
                    status: LoadStatusKind::Complete,
                },
            ),
        ]);
        expected_events.extend(load_events(
            webview,
            &first,
            vec![first.clone(), second.clone()],
            0,
        ));
        expected_events.extend(load_events(
            webview,
            &second,
            vec![first.clone(), second.clone()],
            1,
        ));
        expected_events.extend([
            event(webview, HostEvent::FocusChanged { is_focused: true }),
            event(webview, HostEvent::FocusChanged { is_focused: false }),
        ]);

        assert_eq!(runtime.drain_events(), expected_events);
    }

    #[derive(Default)]
    struct HostDrivenEventsHost {
        created_webviews: HashSet<WebViewHandle>,
    }

    impl Host for HostDrivenEventsHost {
        fn create_webview(
            &mut self,
            _session: SessionHandle,
            webview: WebViewHandle,
        ) -> Result<(), HostError> {
            self.created_webviews.insert(webview);
            Ok(())
        }

        fn attach_surface(
            &mut self,
            webview: WebViewHandle,
            _surface: &HostSurface,
            _size: SurfaceSize,
        ) -> Result<Vec<HostEvent>, HostError> {
            self.ensure_webview(webview)?;
            Ok(vec![HostEvent::CursorChanged {
                cursor: "pointer".to_owned(),
            }])
        }

        fn dispatch_webview_command(
            &mut self,
            webview: WebViewHandle,
            command: &WebViewCommand,
        ) -> Result<Vec<HostEvent>, HostError> {
            self.ensure_webview(webview)?;
            match command {
                WebViewCommand::LoadUrl(request) => Ok(vec![HostEvent::UrlChanged {
                    url: request.url.clone(),
                }]),
                _ => Ok(Vec::new()),
            }
        }

        fn observe_webview_events(
            &mut self,
            webview: WebViewHandle,
            _events: &[HostEvent],
        ) -> Result<(), HostError> {
            self.ensure_webview(webview)
        }

        fn synthesize_webview_command_events(&self) -> bool {
            false
        }
    }

    impl HostDrivenEventsHost {
        fn ensure_webview(&self, webview: WebViewHandle) -> Result<(), HostError> {
            self.created_webviews
                .contains(&webview)
                .then_some(())
                .ok_or_else(|| HostError::new("webview was not created"))
        }
    }

    #[test]
    fn runtime_can_record_host_driven_events_without_synthesizing_command_events() {
        let mut runtime = Runtime::new(HostDrivenEventsHost::default());
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();

        runtime.load_url(webview, "example.com").unwrap();
        runtime
            .attach_surface(
                webview,
                HostSurface::new("host-surface"),
                SurfaceSize::new(320, 240),
            )
            .unwrap();

        assert_eq!(
            runtime.drain_events(),
            vec![
                event(
                    webview,
                    HostEvent::UrlChanged {
                        url: "https://example.com/".to_owned(),
                    },
                ),
                event(
                    webview,
                    HostEvent::SurfaceAttached {
                        size: SurfaceSize::new(320, 240),
                    },
                ),
                event(
                    webview,
                    HostEvent::CursorChanged {
                        cursor: "pointer".to_owned(),
                    },
                ),
            ]
        );
    }

    #[test]
    fn managed_popup_child_surface_lifecycle_is_child_scoped_and_preserves_parent_state() {
        let mut runtime = Runtime::new(PlaceholderHost::default());
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();
        let child_webview_id = "servo-child-1".to_owned();
        let popup_created = HostEvent::PopupCreated {
            parent_webview_id: "servo-parent-1".to_owned(),
            child_webview_id: child_webview_id.clone(),
            parent_url: Some("https://parent.test/".to_owned()),
            target_url: None,
            window_features: None,
            policy: PopupRequestPolicy::ManagedChild,
        };
        let surface = HostSurface::new("popup-child-surface");
        let initial_viewport = SurfaceViewport::new(SurfaceSize::new(320, 240), 2.0);
        let resized_viewport = SurfaceViewport::new(SurfaceSize::new(480, 320), 2.5);

        runtime
            .set_popup_policy(webview, PopupRequestPolicy::ManagedChild)
            .unwrap();
        runtime.host_mut().queue_update_event(popup_created.clone());
        runtime.perform_updates(webview).unwrap();
        runtime
            .attach_managed_child_surface(
                webview,
                child_webview_id.clone(),
                surface.clone(),
                initial_viewport,
            )
            .unwrap();
        assert_eq!(
            runtime.attach_managed_child_surface(
                webview,
                child_webview_id.clone(),
                surface.clone(),
                initial_viewport,
            ),
            Err(RuntimeError::ManagedChildSurfaceAlreadyAttached(
                webview,
                child_webview_id.clone()
            ))
        );
        runtime.host_mut().queue_managed_child_update_event(
            webview,
            child_webview_id.clone(),
            HostEvent::PageTitleChanged {
                title: Some("Child title".to_owned()),
            },
        );
        runtime
            .perform_managed_child_updates(webview, &child_webview_id)
            .unwrap();
        runtime
            .update_managed_child_surface_viewport(webview, &child_webview_id, resized_viewport)
            .unwrap();
        runtime
            .detach_managed_child_surface(webview, &child_webview_id)
            .unwrap();
        assert_eq!(
            runtime.perform_managed_child_updates(webview, &child_webview_id),
            Err(RuntimeError::ManagedChildSurfaceNotAttached(
                webview,
                child_webview_id.clone()
            ))
        );

        runtime.load_url(webview, "example.com").unwrap();

        assert_eq!(
            runtime.host().calls(),
            &[
                HostCall::CreateWebView { session, webview },
                HostCall::SetPopupPolicy {
                    webview,
                    popup_policy: PopupRequestPolicy::ManagedChild,
                },
                HostCall::PerformUpdates { webview },
                HostCall::AttachManagedChildSurface {
                    webview,
                    child_webview_id: child_webview_id.clone(),
                    surface,
                    viewport: initial_viewport,
                },
                HostCall::PerformManagedChildUpdates {
                    webview,
                    child_webview_id: child_webview_id.clone(),
                },
                HostCall::UpdateManagedChildSurfaceViewport {
                    webview,
                    child_webview_id: child_webview_id.clone(),
                    viewport: resized_viewport,
                },
                HostCall::DetachManagedChildSurface {
                    webview,
                    child_webview_id: child_webview_id.clone(),
                },
                HostCall::DispatchWebViewCommand {
                    webview,
                    command: WebViewCommand::LoadUrl(
                        NavigationRequest::new("example.com").unwrap()
                    ),
                },
            ]
        );

        let mut expected_events = vec![
            event(webview, popup_created),
            child_event(
                webview,
                child_webview_id.clone(),
                HostEvent::SurfaceAttached {
                    size: initial_viewport.size,
                },
            ),
            child_event(
                webview,
                child_webview_id.clone(),
                HostEvent::PageTitleChanged {
                    title: Some("Child title".to_owned()),
                },
            ),
            child_event(
                webview,
                child_webview_id.clone(),
                HostEvent::SurfaceResized {
                    size: resized_viewport.size,
                },
            ),
            child_event(webview, child_webview_id, HostEvent::SurfaceDetached),
        ];
        expected_events.extend(load_events(
            webview,
            "https://example.com/",
            vec!["https://example.com/".to_owned()],
            0,
        ));

        assert_eq!(runtime.host().observed_events(), expected_events.as_slice());
        assert_eq!(runtime.drain_events(), expected_events);
    }

    #[test]
    fn default_denied_popup_events_flow_through_runtime_without_child_webview() {
        let popup_event = HostEvent::PopupRequested {
            parent_webview_id: "servo-parent-1".to_owned(),
            parent_url: Some("https://parent.test/".to_owned()),
            target_url: None,
            window_features: None,
            policy: PopupRequestPolicy::DefaultDeny,
        };
        let host = PlaceholderHost::with_update_events([popup_event.clone()]);
        let mut runtime = Runtime::new(host);
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();

        runtime.perform_updates(webview).unwrap();

        assert_eq!(
            runtime.host().calls(),
            &[
                HostCall::CreateWebView { session, webview },
                HostCall::PerformUpdates { webview },
            ]
        );
        let expected_events = vec![event(webview, popup_event)];
        assert_eq!(runtime.host().observed_events(), expected_events.as_slice());
        assert_eq!(runtime.drain_events(), expected_events);
    }

    #[test]
    fn host_updates_are_drained_after_prior_facade_events() {
        let host = PlaceholderHost::with_update_events([
            HostEvent::PageTitleChanged {
                title: Some("ServoKit".to_owned()),
            },
            HostEvent::StatusTextChanged {
                status: Some("ready".to_owned()),
            },
        ]);
        let mut runtime = Runtime::new(host);
        let session = runtime.create_session();
        let webview = runtime.create_webview(session).unwrap();

        runtime.load_url(webview, "example.com").unwrap();
        runtime.perform_updates(webview).unwrap();

        let mut expected_events = load_events(
            webview,
            "https://example.com/",
            vec!["https://example.com/".to_owned()],
            0,
        );
        expected_events.extend([
            event(
                webview,
                HostEvent::PageTitleChanged {
                    title: Some("ServoKit".to_owned()),
                },
            ),
            event(
                webview,
                HostEvent::StatusTextChanged {
                    status: Some("ready".to_owned()),
                },
            ),
        ]);

        assert_eq!(runtime.host().observed_events(), expected_events.as_slice());
        assert_eq!(runtime.drain_events(), expected_events);
        assert!(matches!(
            runtime.host().calls().last(),
            Some(HostCall::PerformUpdates { webview: called }) if *called == webview
        ));

        let calls_after_first_drain = runtime.host().calls().len();
        let observed_events_after_first_drain = runtime.host().observed_events().to_vec();
        runtime.perform_updates(webview).unwrap();

        assert!(runtime.drain_events().is_empty());
        assert_eq!(
            runtime.host().observed_events(),
            observed_events_after_first_drain.as_slice()
        );
        assert_eq!(
            &runtime.host().calls()[calls_after_first_drain..],
            &[HostCall::PerformUpdates { webview }]
        );
    }
}
