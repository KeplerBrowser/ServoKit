use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

use dpi::PhysicalSize;
use euclid::Scale;

use crate::{
    host_event_from_javascript_evaluation_result, ContextMenuAction, ContextMenuBounds,
    ContextMenuElementInformation, ContextMenuItem, ContextMenuRequest, FilePickerRequest,
    HiddenEmbedderControl, HostEvent, InputMethodKind, InputMethodRequest, LoadStatusKind,
    PopupCreated, PopupRequest, PopupRequestPolicy, SelectElementRequest, ServoAdapterState,
    SimpleDialogKind, SimpleDialogRequest,
};
use servo::{
    DeviceIndependentPixel, DevicePixel, InputMethodType, LoadStatus, RenderingContext,
    SimpleDialog, WebView, WebViewDelegate,
};
use servokit_host::{HostSurface, SurfaceSize, SurfaceViewport};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServoWebViewEvent {
    pub webview_id: String,
    pub event: HostEvent,
}

pub type ManagedChildRenderingContextFactory =
    Rc<dyn Fn(Rc<dyn RenderingContext>) -> Rc<dyn RenderingContext>>;

fn default_managed_child_rendering_context_factory() -> ManagedChildRenderingContextFactory {
    Rc::new(|parent_rendering_context| parent_rendering_context)
}

#[derive(Clone)]
pub struct ServoWebViewAdapter {
    shared: Rc<SharedState>,
}

impl ServoWebViewAdapter {
    pub fn new(
        popup_policy: PopupRequestPolicy,
        rendering_context: Rc<dyn RenderingContext>,
    ) -> Self {
        Self {
            shared: Rc::new(SharedState::new(
                popup_policy,
                rendering_context,
                default_managed_child_rendering_context_factory(),
                false,
            )),
        }
    }

    pub fn with_managed_child_rendering_context_factory(
        popup_policy: PopupRequestPolicy,
        rendering_context: Rc<dyn RenderingContext>,
        managed_child_rendering_context_factory: ManagedChildRenderingContextFactory,
    ) -> Self {
        Self {
            shared: Rc::new(SharedState::new(
                popup_policy,
                rendering_context,
                managed_child_rendering_context_factory,
                true,
            )),
        }
    }

    pub fn webview_delegate(&self) -> Rc<dyn WebViewDelegate> {
        Rc::new(ServoWebViewDelegate {
            shared: self.shared.clone(),
        })
    }

    pub fn register_root_webview(&self, webview: &WebView) {
        self.shared
            .manager
            .borrow_mut()
            .register_root_webview(webview_key(webview));
    }

    pub fn request_paint(&self) {
        self.shared.manager.borrow_mut().request_root_paint();
    }

    pub fn take_needs_paint(&self) -> bool {
        self.shared.manager.borrow_mut().take_root_needs_paint()
    }

    pub fn drain_events(&self) -> Vec<HostEvent> {
        self.drain_webview_events()
            .into_iter()
            .map(|event| event.event)
            .collect()
    }

    pub fn drain_webview_events(&self) -> Vec<ServoWebViewEvent> {
        self.shared.manager.borrow_mut().drain_events()
    }

    pub fn set_popup_policy(&self, popup_policy: PopupRequestPolicy) {
        self.shared
            .manager
            .borrow_mut()
            .set_popup_policy(popup_policy);
    }

    pub fn managed_child_webview_ids(&self) -> Vec<String> {
        self.shared.manager.borrow().managed_child_webview_ids()
    }

    pub fn opener_webview_id(&self, child_webview_id: &str) -> Option<String> {
        self.shared
            .manager
            .borrow()
            .opener_webview_id(child_webview_id)
    }

    pub fn destroy_managed_child_webview(&self, child_webview_id: &str) -> Result<(), String> {
        self.shared.destroy_managed_child_webview(child_webview_id)
    }

    pub fn attach_managed_child_surface(
        &self,
        child_webview_id: &str,
        surface: HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<(), String> {
        let child_webview = self
            .shared
            .manager
            .borrow_mut()
            .attach_managed_child_surface(child_webview_id, surface, viewport)?;
        child_webview.set_hidpi_scale_factor(servo_hidpi_scale_factor(viewport.scale_factor));
        child_webview.resize(servo_physical_size(viewport.size));
        Ok(())
    }

    pub fn update_managed_child_surface_viewport(
        &self,
        child_webview_id: &str,
        viewport: SurfaceViewport,
    ) -> Result<(), String> {
        let child_webview = self
            .shared
            .manager
            .borrow_mut()
            .update_managed_child_surface_viewport(child_webview_id, viewport)?;
        child_webview.set_hidpi_scale_factor(servo_hidpi_scale_factor(viewport.scale_factor));
        child_webview.resize(servo_physical_size(viewport.size));
        Ok(())
    }

    pub fn detach_managed_child_surface(&self, child_webview_id: &str) -> Result<(), String> {
        self.shared
            .manager
            .borrow_mut()
            .detach_managed_child_surface(child_webview_id)
    }

    pub fn managed_child_surface_viewport(
        &self,
        child_webview_id: &str,
    ) -> Option<SurfaceViewport> {
        self.shared
            .manager
            .borrow()
            .managed_child_surface_viewport(child_webview_id)
    }

    pub fn managed_child_surface_attached(&self, child_webview_id: &str) -> Result<bool, String> {
        self.shared
            .manager
            .borrow()
            .managed_child_surface_attached(child_webview_id)
    }

    pub fn take_managed_child_needs_paint(&self, child_webview_id: &str) -> Result<bool, String> {
        self.shared
            .manager
            .borrow_mut()
            .take_managed_child_needs_paint(child_webview_id)
    }

    pub fn paint_managed_child(&self, child_webview_id: &str) -> Result<(), String> {
        let child_webview = self
            .shared
            .manager
            .borrow()
            .managed_child_webview(child_webview_id)?;
        child_webview.paint();
        Ok(())
    }

    pub fn ensure_managed_child_webview(&self, child_webview_id: &str) -> Result<(), String> {
        self.shared
            .manager
            .borrow()
            .ensure_managed_child_webview(child_webview_id)
    }

    pub fn notify_javascript_evaluation_result(
        &self,
        evaluation_id: String,
        result: Result<servo::JSValue, servo::JavaScriptEvaluationError>,
    ) {
        let webview_id = self.shared.manager.borrow().root_webview_id();
        match host_event_from_javascript_evaluation_result(evaluation_id, result) {
            HostEvent::JavaScriptEvaluationResult {
                evaluation_id,
                ok,
                value_json,
                error_type,
            } => self.shared.manager.borrow_mut().update_root_state(|state| {
                state.notify_javascript_evaluation_result(evaluation_id, ok, value_json, error_type)
            }),
            _ => {
                unreachable!("javascript evaluation helper should always return evaluation results")
            }
        }
        debug_assert!(
            webview_id.is_some(),
            "root webview should be registered before JavaScript results are delivered"
        );
    }

    pub fn resolve_select_element(
        &self,
        select_element_id: &str,
        selected_options: Vec<usize>,
    ) -> Result<(), String> {
        let mut pending_select_element = self
            .shared
            .pending_select_elements
            .borrow_mut()
            .remove(select_element_id)
            .ok_or_else(|| format!("select element {select_element_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_select_element(select_element_id)?;

        pending_select_element
            .select_element
            .select(selected_options);
        pending_select_element.select_element.submit();

        Ok(())
    }

    pub fn resolve_file_picker(
        &self,
        file_picker_id: &str,
        selected_paths: Vec<String>,
    ) -> Result<(), String> {
        let mut pending_file_picker = self
            .shared
            .pending_file_pickers
            .borrow_mut()
            .remove(file_picker_id)
            .ok_or_else(|| format!("file picker {file_picker_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_file_picker(file_picker_id)?;

        let selected_paths = selected_paths
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        pending_file_picker.file_picker.select(&selected_paths);
        pending_file_picker.file_picker.submit();

        Ok(())
    }

    pub fn dismiss_file_picker(&self, file_picker_id: &str) -> Result<(), String> {
        let pending_file_picker = self
            .shared
            .pending_file_pickers
            .borrow_mut()
            .remove(file_picker_id)
            .ok_or_else(|| format!("file picker {file_picker_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_file_picker(file_picker_id)?;

        pending_file_picker.file_picker.dismiss();

        Ok(())
    }

    pub fn resolve_context_menu(
        &self,
        context_menu_id: &str,
        action: ContextMenuAction,
    ) -> Result<(), String> {
        let pending_context_menu = self
            .shared
            .pending_context_menus
            .borrow_mut()
            .remove(context_menu_id)
            .ok_or_else(|| format!("context menu {context_menu_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_context_menu(context_menu_id)?;

        pending_context_menu
            .context_menu
            .select(convert_context_menu_action(action));

        Ok(())
    }

    pub fn dismiss_context_menu(&self, context_menu_id: &str) -> Result<(), String> {
        let pending_context_menu = self
            .shared
            .pending_context_menus
            .borrow_mut()
            .remove(context_menu_id)
            .ok_or_else(|| format!("context menu {context_menu_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_context_menu(context_menu_id)?;

        pending_context_menu.context_menu.dismiss();

        Ok(())
    }

    pub fn has_pending_permission_request(&self) -> bool {
        self.shared
            .manager
            .borrow()
            .pending_permission_request()
            .is_some()
    }

    pub fn pending_permission_request(&self) -> Option<(String, String)> {
        self.shared
            .manager
            .borrow()
            .pending_permission_request()
            .map(|request| (request.permission.clone(), request.origin.clone()))
    }

    pub fn resolve_permission(&self, allow: bool) -> Result<(), String> {
        let request = self
            .shared
            .pending_permission_request
            .borrow_mut()
            .take()
            .ok_or_else(|| "permission request is no longer pending".to_owned())?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_permission(&request.webview_id)?;

        if allow {
            request.request.allow();
        } else {
            request.request.deny();
        }

        Ok(())
    }

    pub fn resolve_navigation_request(
        &self,
        navigation_id: &str,
        allow: bool,
    ) -> Result<(), String> {
        let navigation_request = self
            .shared
            .pending_navigation_requests
            .borrow_mut()
            .remove(navigation_id)
            .ok_or_else(|| format!("navigation request {navigation_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_navigation_request(&navigation_request.webview_id, navigation_id)?;

        if allow {
            navigation_request.request.allow();
        } else {
            navigation_request.request.deny();
        }

        Ok(())
    }

    pub fn resolve_simple_dialog(
        &self,
        dialog_id: &str,
        confirmed: bool,
        prompt_value: Option<&str>,
    ) -> Result<(), String> {
        let pending_dialog = self
            .shared
            .pending_simple_dialogs
            .borrow_mut()
            .remove(dialog_id)
            .ok_or_else(|| format!("simple dialog {dialog_id} is no longer pending"))?;
        self.shared
            .manager
            .borrow_mut()
            .resolve_simple_dialog(dialog_id)?;

        match pending_dialog.dialog {
            SimpleDialog::Alert(alert_dialog) => alert_dialog.confirm(),
            SimpleDialog::Confirm(confirm_dialog) => {
                if confirmed {
                    confirm_dialog.confirm();
                } else {
                    confirm_dialog.dismiss();
                }
            }
            SimpleDialog::Prompt(mut prompt_dialog) => {
                if confirmed {
                    if let Some(prompt_value) = prompt_value {
                        prompt_dialog.set_current_value(prompt_value);
                    }
                    prompt_dialog.confirm();
                } else {
                    prompt_dialog.dismiss();
                }
            }
        }

        Ok(())
    }
}

struct SharedState {
    rendering_context: Rc<dyn RenderingContext>,
    managed_child_rendering_context_factory: ManagedChildRenderingContextFactory,
    managed_child_rendering_context_is_child_scoped: bool,
    manager: RefCell<WebViewManager>,
    pending_navigation_requests: RefCell<HashMap<String, PendingNavigationRequest>>,
    pending_simple_dialogs: RefCell<HashMap<String, PendingSimpleDialog>>,
    pending_select_elements: RefCell<HashMap<String, PendingSelectElement>>,
    pending_context_menus: RefCell<HashMap<String, PendingContextMenu>>,
    pending_file_pickers: RefCell<HashMap<String, PendingFilePicker>>,
    pending_permission_request: RefCell<Option<PendingPermissionRequest>>,
}

impl SharedState {
    fn new(
        popup_policy: PopupRequestPolicy,
        rendering_context: Rc<dyn RenderingContext>,
        managed_child_rendering_context_factory: ManagedChildRenderingContextFactory,
        managed_child_rendering_context_is_child_scoped: bool,
    ) -> Self {
        Self {
            rendering_context,
            managed_child_rendering_context_factory,
            managed_child_rendering_context_is_child_scoped,
            manager: RefCell::new(WebViewManager::new(popup_policy)),
            pending_navigation_requests: RefCell::default(),
            pending_simple_dialogs: RefCell::default(),
            pending_select_elements: RefCell::default(),
            pending_context_menus: RefCell::default(),
            pending_file_pickers: RefCell::default(),
            pending_permission_request: RefCell::default(),
        }
    }

    fn destroy_managed_child_webview(&self, child_webview_id: &str) -> Result<(), String> {
        self.cleanup_pending_for_webview(child_webview_id);
        self.manager
            .borrow_mut()
            .destroy_managed_child_webview(child_webview_id)
    }

    fn managed_child_rendering_context(&self) -> Rc<dyn RenderingContext> {
        (self.managed_child_rendering_context_factory)(self.rendering_context.clone())
    }

    fn notify_closed(&self, webview_id: &str) {
        self.cleanup_pending_for_webview(webview_id);
        self.manager.borrow_mut().notify_closed(webview_id);
    }

    fn cleanup_pending_for_webview(&self, webview_id: &str) {
        // Managed child handles may be destroyed by the host or closed by Servo while
        // request-style callbacks are still pending. Clear ServoKit state and answer
        // retained Servo requests synchronously here so stale child controls cannot be
        // resolved later against a removed child. Servo's SelectElement has no dismiss
        // API; dropping it sends Servo's default current-selection response.
        for (navigation_id, navigation_request) in
            self.remove_pending_navigation_requests_for_webview(webview_id)
        {
            let _ = self
                .manager
                .borrow_mut()
                .resolve_navigation_request(&navigation_request.webview_id, &navigation_id);
            navigation_request.request.deny();
        }

        if let Some(permission_request) =
            self.remove_pending_permission_request_for_webview(webview_id)
        {
            let _ = self
                .manager
                .borrow_mut()
                .resolve_permission(&permission_request.webview_id);
            permission_request.request.deny();
        }

        for (dialog_id, simple_dialog) in self.remove_pending_simple_dialogs_for_webview(webview_id)
        {
            let _ = self
                .manager
                .borrow_mut()
                .with_state_for_webview(&simple_dialog.webview_id, |state| {
                    state.resolve_simple_dialog(&dialog_id)
                });
            simple_dialog.dialog.dismiss();
        }

        for (select_element_id, select_element) in
            self.remove_pending_select_elements_for_webview(webview_id)
        {
            let _ = self
                .manager
                .borrow_mut()
                .with_state_for_webview(&select_element.webview_id, |state| {
                    state.resolve_select_element(&select_element_id)
                });
            drop(select_element.select_element);
        }

        for (context_menu_id, context_menu) in
            self.remove_pending_context_menus_for_webview(webview_id)
        {
            let _ = self
                .manager
                .borrow_mut()
                .with_state_for_webview(&context_menu.webview_id, |state| {
                    state.resolve_context_menu(&context_menu_id)
                });
            context_menu.context_menu.dismiss();
        }

        for (file_picker_id, file_picker) in
            self.remove_pending_file_pickers_for_webview(webview_id)
        {
            let _ = self
                .manager
                .borrow_mut()
                .with_state_for_webview(&file_picker.webview_id, |state| {
                    state.resolve_file_picker(&file_picker_id)
                });
            file_picker.file_picker.dismiss();
        }
    }

    fn remove_pending_navigation_requests_for_webview(
        &self,
        webview_id: &str,
    ) -> Vec<(String, PendingNavigationRequest)> {
        let ids = pending_ids_for_webview(&self.pending_navigation_requests.borrow(), webview_id);
        remove_pending_by_ids(&mut *self.pending_navigation_requests.borrow_mut(), ids)
    }

    fn remove_pending_permission_request_for_webview(
        &self,
        webview_id: &str,
    ) -> Option<PendingPermissionRequest> {
        let mut pending_permission_request = self.pending_permission_request.borrow_mut();
        pending_permission_request
            .as_ref()
            .is_some_and(|request| request.webview_id == webview_id)
            .then(|| pending_permission_request.take())
            .flatten()
    }

    fn remove_pending_simple_dialogs_for_webview(
        &self,
        webview_id: &str,
    ) -> Vec<(String, PendingSimpleDialog)> {
        let ids = pending_ids_for_webview(&self.pending_simple_dialogs.borrow(), webview_id);
        remove_pending_by_ids(&mut *self.pending_simple_dialogs.borrow_mut(), ids)
    }

    fn remove_pending_select_elements_for_webview(
        &self,
        webview_id: &str,
    ) -> Vec<(String, PendingSelectElement)> {
        let ids = pending_ids_for_webview(&self.pending_select_elements.borrow(), webview_id);
        remove_pending_by_ids(&mut *self.pending_select_elements.borrow_mut(), ids)
    }

    fn remove_pending_context_menus_for_webview(
        &self,
        webview_id: &str,
    ) -> Vec<(String, PendingContextMenu)> {
        let ids = pending_ids_for_webview(&self.pending_context_menus.borrow(), webview_id);
        remove_pending_by_ids(&mut *self.pending_context_menus.borrow_mut(), ids)
    }

    fn remove_pending_file_pickers_for_webview(
        &self,
        webview_id: &str,
    ) -> Vec<(String, PendingFilePicker)> {
        let ids = pending_ids_for_webview(&self.pending_file_pickers.borrow(), webview_id);
        remove_pending_by_ids(&mut *self.pending_file_pickers.borrow_mut(), ids)
    }
}

struct WebViewManager {
    popup_policy: PopupRequestPolicy,
    root_webview_id: Option<String>,
    root_state: ServoAdapterState,
    child_states: HashMap<String, ServoAdapterState>,
    managed_children: HashMap<String, ManagedChildWebView>,
    events: VecDeque<ServoWebViewEvent>,
    next_navigation_request_id: u64,
}

impl WebViewManager {
    fn new(popup_policy: PopupRequestPolicy) -> Self {
        Self {
            popup_policy,
            root_webview_id: None,
            root_state: ServoAdapterState::default(),
            child_states: HashMap::new(),
            managed_children: HashMap::new(),
            events: VecDeque::new(),
            next_navigation_request_id: 0,
        }
    }

    fn popup_policy(&self) -> PopupRequestPolicy {
        self.popup_policy
    }

    fn set_popup_policy(&mut self, popup_policy: PopupRequestPolicy) {
        self.popup_policy = popup_policy;
    }

    fn root_webview_id(&self) -> Option<String> {
        self.root_webview_id.clone()
    }

    fn register_root_webview(&mut self, webview_id: String) {
        self.root_webview_id = Some(webview_id);
    }

    fn request_root_paint(&mut self) {
        self.root_state.request_paint();
    }

    fn take_root_needs_paint(&mut self) -> bool {
        self.root_state.take_needs_paint()
    }

    fn current_url(&self, webview_id: &str) -> Option<String> {
        if self.is_root_webview(webview_id) {
            self.root_state.current_url().map(str::to_owned)
        } else {
            self.child_states
                .get(webview_id)
                .and_then(|state| state.current_url().map(str::to_owned))
        }
    }

    fn managed_child_webview_ids(&self) -> Vec<String> {
        let mut ids = self
            .managed_children
            .values()
            .map(|child| child.webview.id().to_string())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    fn opener_webview_id(&self, child_webview_id: &str) -> Option<String> {
        self.managed_children
            .get(child_webview_id)
            .map(|child| child.parent_webview_id.clone())
    }

    fn register_managed_child(
        &mut self,
        parent_webview_id: String,
        child_webview: WebView,
        parent_url: Option<String>,
        surface_adoptable: bool,
    ) -> String {
        let child_webview_id = webview_key(&child_webview);
        self.child_states
            .entry(child_webview_id.clone())
            .or_insert_with(ServoAdapterState::default);
        self.managed_children.insert(
            child_webview_id.clone(),
            ManagedChildWebView {
                parent_webview_id: parent_webview_id.clone(),
                webview: child_webview,
                surface: None,
                surface_adoptable,
            },
        );
        let popup_parent_webview_id = parent_webview_id.clone();
        let popup_child_webview_id = child_webview_id.clone();
        self.update_state_for_webview(&parent_webview_id, |state| {
            state.notify_popup_created(PopupCreated {
                parent_webview_id: popup_parent_webview_id,
                child_webview_id: popup_child_webview_id,
                parent_url,
                target_url: None,
                window_features: None,
            });
        });
        child_webview_id
    }

    fn destroy_managed_child_webview(&mut self, child_webview_id: &str) -> Result<(), String> {
        self.managed_children
            .remove(child_webview_id)
            .map(|_| ())
            .ok_or_else(|| format!("managed child webview {child_webview_id} is not registered"))?;
        self.child_states.remove(child_webview_id);
        Ok(())
    }

    fn ensure_managed_child_webview(&self, child_webview_id: &str) -> Result<(), String> {
        self.managed_children
            .contains_key(child_webview_id)
            .then_some(())
            .ok_or_else(|| format!("managed child webview {child_webview_id} is not registered"))
    }

    fn managed_child_webview(&self, child_webview_id: &str) -> Result<WebView, String> {
        self.managed_children
            .get(child_webview_id)
            .map(|child| child.webview.clone())
            .ok_or_else(|| format!("managed child webview {child_webview_id} is not registered"))
    }

    fn attach_managed_child_surface(
        &mut self,
        child_webview_id: &str,
        surface: HostSurface,
        viewport: SurfaceViewport,
    ) -> Result<WebView, String> {
        let child_webview = {
            let child = self
                .managed_children
                .get_mut(child_webview_id)
                .ok_or_else(|| {
                    format!("managed child webview {child_webview_id} is not registered")
                })?;
            if !child.surface_adoptable {
                return Err(format!(
                    "managed child webview {child_webview_id} does not have a child-scoped rendering context"
                ));
            }
            if child.surface.is_some() {
                return Err(format!(
                    "managed child webview {child_webview_id} already has an attached surface"
                ));
            }
            child.surface = Some(AttachedManagedChildSurface {
                _surface: surface,
                viewport,
            });
            child.webview.clone()
        };

        self.update_state_for_webview(child_webview_id, |state| {
            state.notify_surface_attached(viewport.size);
            state.request_paint();
        });
        Ok(child_webview)
    }

    fn update_managed_child_surface_viewport(
        &mut self,
        child_webview_id: &str,
        viewport: SurfaceViewport,
    ) -> Result<WebView, String> {
        let child_webview = {
            let child = self
                .managed_children
                .get_mut(child_webview_id)
                .ok_or_else(|| {
                    format!("managed child webview {child_webview_id} is not registered")
                })?;
            let Some(attached_surface) = child.surface.as_mut() else {
                return Err(format!(
                    "managed child webview {child_webview_id} does not have an attached surface"
                ));
            };
            attached_surface.viewport = viewport;
            child.webview.clone()
        };

        self.update_state_for_webview(child_webview_id, |state| {
            state.notify_surface_resized(viewport.size);
            state.request_paint();
        });
        Ok(child_webview)
    }

    fn detach_managed_child_surface(&mut self, child_webview_id: &str) -> Result<(), String> {
        let removed = {
            let child = self
                .managed_children
                .get_mut(child_webview_id)
                .ok_or_else(|| {
                    format!("managed child webview {child_webview_id} is not registered")
                })?;
            child.surface.take()
        };
        if removed.is_none() {
            return Err(format!(
                "managed child webview {child_webview_id} does not have an attached surface"
            ));
        }

        self.update_state_for_webview(child_webview_id, |state| {
            state.notify_surface_detached();
        });
        Ok(())
    }

    fn managed_child_surface_viewport(&self, child_webview_id: &str) -> Option<SurfaceViewport> {
        self.managed_children
            .get(child_webview_id)
            .and_then(|child| child.surface.as_ref())
            .map(|surface| surface.viewport)
    }

    fn managed_child_surface_attached(&self, child_webview_id: &str) -> Result<bool, String> {
        self.managed_children
            .get(child_webview_id)
            .map(|child| child.surface.is_some())
            .ok_or_else(|| format!("managed child webview {child_webview_id} is not registered"))
    }

    fn take_managed_child_needs_paint(&mut self, child_webview_id: &str) -> Result<bool, String> {
        self.child_states
            .get_mut(child_webview_id)
            .map(ServoAdapterState::take_needs_paint)
            .ok_or_else(|| format!("managed child webview {child_webview_id} is not registered"))
    }

    fn notify_closed(&mut self, webview_id: &str) {
        self.update_state_for_webview(webview_id, |state| state.notify_closed());
        if self.managed_children.remove(webview_id).is_some() {
            self.child_states.remove(webview_id);
        }
    }

    fn request_popup(&mut self, parent_webview_id: &str, parent_url: Option<String>) {
        self.update_state_for_webview(parent_webview_id, |state| {
            state.request_popup(PopupRequest {
                parent_webview_id: parent_webview_id.to_owned(),
                parent_url,
                target_url: None,
                window_features: None,
            });
        });
    }

    fn request_navigation(&mut self, webview_id: &str, url: String) -> String {
        let navigation_id = self.next_navigation_request_id();
        let routed_navigation_id = navigation_id.clone();
        self.update_state_for_webview(webview_id, |state| {
            state.request_navigation_with_id(routed_navigation_id, url);
        });
        navigation_id
    }

    fn next_navigation_request_id(&mut self) -> String {
        self.next_navigation_request_id = self.next_navigation_request_id.wrapping_add(1);
        format!("navigation-{}", self.next_navigation_request_id)
    }

    fn resolve_navigation_request(
        &mut self,
        webview_id: &str,
        navigation_id: &str,
    ) -> Result<(), String> {
        self.with_state_for_webview(webview_id, |state| {
            state.resolve_navigation_request(navigation_id)
        })
    }

    fn resolve_select_element(&mut self, select_element_id: &str) -> Result<(), String> {
        self.resolve_in_any_state(|state| state.resolve_select_element(select_element_id))
    }

    fn resolve_file_picker(&mut self, file_picker_id: &str) -> Result<(), String> {
        self.resolve_in_any_state(|state| state.resolve_file_picker(file_picker_id))
    }

    fn resolve_context_menu(&mut self, context_menu_id: &str) -> Result<(), String> {
        self.resolve_in_any_state(|state| state.resolve_context_menu(context_menu_id))
    }

    fn resolve_simple_dialog(&mut self, dialog_id: &str) -> Result<(), String> {
        self.resolve_in_any_state(|state| state.resolve_simple_dialog(dialog_id))
    }

    fn hide_embedder_control(
        &mut self,
        webview_id: &str,
        control_id: &str,
    ) -> HiddenEmbedderControl {
        let mut hidden = HiddenEmbedderControl::Unknown;
        self.update_state_for_webview(webview_id, |state| {
            hidden = state.hide_embedder_control(control_id);
        });
        hidden
    }

    fn resolve_permission(&mut self, webview_id: &str) -> Result<(), String> {
        self.with_state_for_webview(webview_id, ServoAdapterState::resolve_permission)
    }

    fn pending_permission_request(&self) -> Option<crate::PendingPermissionRequestInfo> {
        self.root_state.pending_permission_request().or_else(|| {
            self.child_states
                .values()
                .find_map(ServoAdapterState::pending_permission_request)
        })
    }

    fn update_root_state(&mut self, update: impl FnOnce(&mut ServoAdapterState)) {
        let webview_id = self.root_webview_id.clone().unwrap_or_default();
        update(&mut self.root_state);
        self.drain_state_events(webview_id, false);
    }

    fn update_state_for_webview(
        &mut self,
        webview_id: &str,
        update: impl FnOnce(&mut ServoAdapterState),
    ) {
        if self.is_root_webview(webview_id) {
            update(&mut self.root_state);
            self.drain_state_events(webview_id.to_owned(), false);
            return;
        }

        let state = self
            .child_states
            .entry(webview_id.to_owned())
            .or_insert_with(ServoAdapterState::default);
        update(state);
        self.drain_state_events(webview_id.to_owned(), true);
    }

    fn with_state_for_webview(
        &mut self,
        webview_id: &str,
        update: impl FnOnce(&mut ServoAdapterState) -> Result<(), String>,
    ) -> Result<(), String> {
        if self.is_root_webview(webview_id) {
            return update(&mut self.root_state);
        }
        let Some(state) = self.child_states.get_mut(webview_id) else {
            return Err(format!("webview {webview_id} is not registered"));
        };
        update(state)
    }

    fn resolve_in_any_state(
        &mut self,
        mut update: impl FnMut(&mut ServoAdapterState) -> Result<(), String>,
    ) -> Result<(), String> {
        match update(&mut self.root_state) {
            Ok(()) => return Ok(()),
            Err(mut last_error) => {
                for state in self.child_states.values_mut() {
                    match update(state) {
                        Ok(()) => return Ok(()),
                        Err(error) => last_error = error,
                    }
                }
                Err(last_error)
            }
        }
    }

    fn drain_state_events(&mut self, webview_id: String, child: bool) {
        let events = if child {
            self.child_states
                .get_mut(&webview_id)
                .map(ServoAdapterState::drain_events)
                .unwrap_or_default()
        } else {
            self.root_state.drain_events()
        };
        self.events
            .extend(events.into_iter().map(|event| ServoWebViewEvent {
                webview_id: webview_id.clone(),
                event,
            }));
    }

    fn drain_events(&mut self) -> Vec<ServoWebViewEvent> {
        self.events.drain(..).collect()
    }

    fn is_root_webview(&self, webview_id: &str) -> bool {
        self.root_webview_id
            .as_deref()
            .map(|root| root == webview_id)
            .unwrap_or(true)
    }
}

struct AttachedManagedChildSurface {
    _surface: HostSurface,
    viewport: SurfaceViewport,
}

struct ManagedChildWebView {
    parent_webview_id: String,
    webview: WebView,
    surface: Option<AttachedManagedChildSurface>,
    surface_adoptable: bool,
}

trait PendingWebViewScoped {
    fn webview_id(&self) -> &str;
}

struct PendingNavigationRequest {
    webview_id: String,
    request: servo::NavigationRequest,
}

impl PendingWebViewScoped for PendingNavigationRequest {
    fn webview_id(&self) -> &str {
        &self.webview_id
    }
}

struct PendingSimpleDialog {
    webview_id: String,
    dialog: SimpleDialog,
}

impl PendingWebViewScoped for PendingSimpleDialog {
    fn webview_id(&self) -> &str {
        &self.webview_id
    }
}

struct PendingSelectElement {
    webview_id: String,
    select_element: servo::SelectElement,
}

impl PendingWebViewScoped for PendingSelectElement {
    fn webview_id(&self) -> &str {
        &self.webview_id
    }
}

struct PendingContextMenu {
    webview_id: String,
    context_menu: servo::ContextMenu,
}

impl PendingWebViewScoped for PendingContextMenu {
    fn webview_id(&self) -> &str {
        &self.webview_id
    }
}

struct PendingFilePicker {
    webview_id: String,
    file_picker: servo::FilePicker,
}

impl PendingWebViewScoped for PendingFilePicker {
    fn webview_id(&self) -> &str {
        &self.webview_id
    }
}

fn pending_ids_for_webview<T: PendingWebViewScoped>(
    pending: &HashMap<String, T>,
    webview_id: &str,
) -> Vec<String> {
    pending
        .iter()
        .filter(|(_, request)| request.webview_id() == webview_id)
        .map(|(id, _)| id.clone())
        .collect()
}

fn remove_pending_by_ids<T>(
    pending: &mut HashMap<String, T>,
    ids: Vec<String>,
) -> Vec<(String, T)> {
    ids.into_iter()
        .filter_map(|id| pending.remove(&id).map(|request| (id, request)))
        .collect()
}

fn servo_physical_size(size: SurfaceSize) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width, size.height)
}

fn servo_hidpi_scale_factor(density: f32) -> Scale<f32, DeviceIndependentPixel, DevicePixel> {
    Scale::new(density)
}

struct ServoWebViewDelegate {
    shared: Rc<SharedState>,
}

struct PendingPermissionRequest {
    webview_id: String,
    request: servo::PermissionRequest,
}

impl PendingWebViewScoped for PendingPermissionRequest {
    fn webview_id(&self) -> &str {
        &self.webview_id
    }
}

impl servo::WebViewDelegate for ServoWebViewDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| state.request_paint());
    }

    fn notify_url_changed(&self, webview: WebView, url: Url) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| state.notify_url_changed(url.as_str()));
    }

    fn notify_page_title_changed(&self, webview: WebView, title: Option<String>) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| state.notify_page_title_changed(title));
    }

    fn notify_status_text_changed(&self, webview: WebView, status: Option<String>) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.notify_status_text_changed(status)
            });
    }

    fn notify_focus_changed(&self, webview: WebView, focused: bool) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| state.notify_focus_changed(focused));
    }

    fn notify_load_status_changed(&self, webview: WebView, status: LoadStatus) {
        let status = match status {
            LoadStatus::Started => LoadStatusKind::Started,
            LoadStatus::HeadParsed => LoadStatusKind::HeadParsed,
            LoadStatus::Complete => LoadStatusKind::Complete,
        };
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.notify_load_status_changed(status)
            });
    }

    fn notify_cursor_changed(&self, webview: WebView, cursor: servo::Cursor) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.notify_cursor_changed(cursor_value(cursor))
            });
    }

    fn notify_history_changed(&self, webview: WebView, entries: Vec<Url>, current: usize) {
        let webview_id = webview_key(&webview);
        let can_go_back = webview.can_go_back();
        let can_go_forward = webview.can_go_forward();
        let entries = entries.into_iter().map(|entry| entry.to_string()).collect();
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.notify_history_changed(entries, current, can_go_back, can_go_forward)
            });
    }

    fn notify_closed(&self, webview: WebView) {
        let webview_id = webview_key(&webview);
        self.shared.notify_closed(&webview_id);
    }

    fn notify_fullscreen_state_changed(&self, webview: WebView, is_fullscreen: bool) {
        let webview_id = webview_key(&webview);
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.notify_fullscreen_state_changed(is_fullscreen)
            });
    }

    fn notify_crashed(&self, webview: WebView, reason: String, backtrace: Option<String>) {
        let webview_id = webview_key(&webview);
        let webview_url = webview.url().map(|url| url.to_string());
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.notify_crashed(webview_url.as_deref(), reason, backtrace)
            });
    }

    fn request_navigation(&self, webview: WebView, request: servo::NavigationRequest) {
        let webview_id = webview_key(&webview);
        let url = request.url.to_string();
        let navigation_id = self
            .shared
            .manager
            .borrow_mut()
            .request_navigation(&webview_id, url);

        self.shared.pending_navigation_requests.borrow_mut().insert(
            navigation_id.clone(),
            PendingNavigationRequest {
                webview_id,
                request,
            },
        );
    }

    fn request_create_new(&self, parent_webview: WebView, request: servo::CreateNewWebViewRequest) {
        let parent_webview_id = webview_key(&parent_webview);
        let parent_url = parent_webview
            .url()
            .map(|url| url.to_string())
            .or_else(|| self.shared.manager.borrow().current_url(&parent_webview_id));

        let popup_policy = self.shared.manager.borrow().popup_policy();
        match popup_policy {
            PopupRequestPolicy::DefaultDeny => {
                // Do not synthesize target URL/window-feature metadata, and do not retain the
                // request across an async host bridge. Dropping it applies Servo's default-deny
                // response (None) and creates no child WebView.
                self.shared
                    .manager
                    .borrow_mut()
                    .request_popup(&parent_webview_id, parent_url);
                drop(request);
            }
            PopupRequestPolicy::ManagedChild => {
                let child_rendering_context = self.shared.managed_child_rendering_context();
                let child_webview = request
                    .builder(child_rendering_context)
                    .hidpi_scale_factor(parent_webview.hidpi_scale_factor())
                    .clipboard_delegate(parent_webview.clipboard_delegate())
                    .delegate(parent_webview.delegate())
                    .build();
                self.shared.manager.borrow_mut().register_managed_child(
                    parent_webview_id,
                    child_webview,
                    parent_url,
                    self.shared.managed_child_rendering_context_is_child_scoped,
                );
            }
        }
    }

    fn request_permission(&self, webview: WebView, request: servo::PermissionRequest) {
        let webview_id = webview_key(&webview);
        let permission = permission_feature_name(request.feature());
        let origin = permission_request_origin(&webview)
            .or_else(|| self.shared.manager.borrow().current_url(&webview_id))
            .unwrap_or_default();

        // Deny any previously pending request before replacing.
        if let Some(old) = self.shared.pending_permission_request.borrow_mut().take() {
            let _ = self
                .shared
                .manager
                .borrow_mut()
                .resolve_permission(&old.webview_id);
            old.request.deny();
        }

        self.shared
            .pending_permission_request
            .borrow_mut()
            .replace(PendingPermissionRequest {
                webview_id: webview_id.clone(),
                request,
            });
        self.shared
            .manager
            .borrow_mut()
            .update_state_for_webview(&webview_id, |state| {
                state.request_permission(permission, origin);
            });
    }

    fn show_embedder_control(&self, webview: WebView, embedder_control: servo::EmbedderControl) {
        let webview_id = webview_key(&webview);
        let control_id = embedder_control_key(embedder_control.id());

        match embedder_control {
            servo::EmbedderControl::InputMethod(input_method) => {
                self.shared
                    .manager
                    .borrow_mut()
                    .update_state_for_webview(&webview_id, |state| {
                        state.show_input_method(InputMethodRequest {
                            input_method_id: control_id,
                            input_method_type: input_method_kind(input_method.input_method_type()),
                            text: input_method.text(),
                            insertion_point: input_method.insertion_point(),
                            multiline: input_method.multiline(),
                            allow_virtual_keyboard: input_method.allow_virtual_keyboard(),
                        });
                    });
            }
            servo::EmbedderControl::SelectElement(select_element) => {
                self.shared.pending_select_elements.borrow_mut().insert(
                    control_id.clone(),
                    PendingSelectElement {
                        webview_id: webview_id.clone(),
                        select_element,
                    },
                );

                let select_element_ref = self.shared.pending_select_elements.borrow();
                let select_element = &select_element_ref.get(&control_id).unwrap().select_element;

                let options = select_element
                    .options()
                    .iter()
                    .map(|opt| convert_select_element_option(opt))
                    .collect();
                let selected_options = select_element.selected_options();
                let allow_select_multiple = select_element.allow_select_multiple();

                self.shared
                    .manager
                    .borrow_mut()
                    .update_state_for_webview(&webview_id, |state| {
                        state.show_select_element(SelectElementRequest {
                            select_element_id: control_id,
                            options,
                            selected_options,
                            allow_select_multiple,
                        });
                    });
            }
            servo::EmbedderControl::ContextMenu(context_menu) => {
                let position = context_menu.position();
                let element_info = convert_context_menu_element_info(context_menu.element_info());
                let items = context_menu
                    .items()
                    .iter()
                    .map(convert_context_menu_item)
                    .collect();
                let bounds = ContextMenuBounds::new(
                    position.min.x,
                    position.min.y,
                    position.size().width,
                    position.size().height,
                );

                self.shared.pending_context_menus.borrow_mut().insert(
                    control_id.clone(),
                    PendingContextMenu {
                        webview_id: webview_id.clone(),
                        context_menu,
                    },
                );

                self.shared
                    .manager
                    .borrow_mut()
                    .update_state_for_webview(&webview_id, |state| {
                        state.show_context_menu(ContextMenuRequest {
                            context_menu_id: control_id,
                            bounds,
                            element_info,
                            items,
                        });
                    });
            }
            servo::EmbedderControl::FilePicker(file_picker) => {
                self.shared.pending_file_pickers.borrow_mut().insert(
                    control_id.clone(),
                    PendingFilePicker {
                        webview_id: webview_id.clone(),
                        file_picker,
                    },
                );

                let file_picker_ref = self.shared.pending_file_pickers.borrow();
                let file_picker = &file_picker_ref.get(&control_id).unwrap().file_picker;
                let current_paths = file_picker
                    .current_paths()
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                let filter_patterns = file_picker
                    .filter_patterns()
                    .iter()
                    .map(|pattern| pattern.0.clone())
                    .collect();
                let allow_select_multiple = file_picker.allow_select_multiple();

                self.shared
                    .manager
                    .borrow_mut()
                    .update_state_for_webview(&webview_id, |state| {
                        state.show_file_picker(FilePickerRequest {
                            file_picker_id: control_id,
                            current_paths,
                            filter_patterns,
                            allow_select_multiple,
                        });
                    });
            }
            servo::EmbedderControl::SimpleDialog(dialog) => {
                let kind = simple_dialog_kind(&dialog);
                let message = dialog.message().to_owned();
                let default_value = simple_dialog_default_value(&dialog);
                self.shared.pending_simple_dialogs.borrow_mut().insert(
                    control_id.clone(),
                    PendingSimpleDialog {
                        webview_id: webview_id.clone(),
                        dialog,
                    },
                );
                self.shared
                    .manager
                    .borrow_mut()
                    .update_state_for_webview(&webview_id, |state| {
                        state.show_simple_dialog(SimpleDialogRequest {
                            dialog_id: control_id,
                            kind,
                            message,
                            default_value,
                        });
                    });
            }
            _ => {}
        }
    }

    fn hide_embedder_control(&self, webview: WebView, control_id: servo::EmbedderControlId) {
        let webview_id = webview_key(&webview);
        let control_id = embedder_control_key(control_id);
        let hidden_control = self
            .shared
            .manager
            .borrow_mut()
            .hide_embedder_control(&webview_id, &control_id);
        match hidden_control {
            HiddenEmbedderControl::InputMethod => {}
            HiddenEmbedderControl::SelectElement => {
                self.shared
                    .pending_select_elements
                    .borrow_mut()
                    .remove(&control_id);
            }
            HiddenEmbedderControl::FilePicker => {
                self.shared
                    .pending_file_pickers
                    .borrow_mut()
                    .remove(&control_id);
            }
            HiddenEmbedderControl::ContextMenu => {
                self.shared
                    .pending_context_menus
                    .borrow_mut()
                    .remove(&control_id);
            }
            HiddenEmbedderControl::SimpleDialog => {
                self.shared
                    .pending_simple_dialogs
                    .borrow_mut()
                    .remove(&control_id);
            }
            HiddenEmbedderControl::Unknown => {}
        }
    }
}

fn webview_key(webview: &WebView) -> String {
    webview.id().to_string()
}

fn embedder_control_key(control_id: servo::EmbedderControlId) -> String {
    format!(
        "{:?}:{:?}:{:?}",
        control_id.webview_id, control_id.pipeline_id, control_id.index
    )
}

fn simple_dialog_kind(dialog: &SimpleDialog) -> SimpleDialogKind {
    match dialog {
        SimpleDialog::Alert(_) => SimpleDialogKind::Alert,
        SimpleDialog::Confirm(_) => SimpleDialogKind::Confirm,
        SimpleDialog::Prompt(_) => SimpleDialogKind::Prompt,
    }
}

fn simple_dialog_default_value(dialog: &SimpleDialog) -> Option<String> {
    match dialog {
        SimpleDialog::Prompt(prompt_dialog) => Some(prompt_dialog.current_value().to_owned()),
        SimpleDialog::Alert(_) | SimpleDialog::Confirm(_) => None,
    }
}

fn input_method_kind(input_method_type: InputMethodType) -> InputMethodKind {
    match input_method_type {
        InputMethodType::Color => InputMethodKind::Color,
        InputMethodType::Date => InputMethodKind::Date,
        InputMethodType::DatetimeLocal => InputMethodKind::DatetimeLocal,
        InputMethodType::Email => InputMethodKind::Email,
        InputMethodType::Month => InputMethodKind::Month,
        InputMethodType::Number => InputMethodKind::Number,
        InputMethodType::Password => InputMethodKind::Password,
        InputMethodType::Search => InputMethodKind::Search,
        InputMethodType::Tel => InputMethodKind::Tel,
        InputMethodType::Text => InputMethodKind::Text,
        InputMethodType::Time => InputMethodKind::Time,
        InputMethodType::Url => InputMethodKind::Url,
        InputMethodType::Week => InputMethodKind::Week,
    }
}

fn cursor_value(cursor: servo::Cursor) -> &'static str {
    match cursor {
        servo::Cursor::None => "none",
        servo::Cursor::Default => "default",
        servo::Cursor::Pointer => "pointer",
        servo::Cursor::ContextMenu => "context-menu",
        servo::Cursor::Help => "help",
        servo::Cursor::Progress => "progress",
        servo::Cursor::Wait => "wait",
        servo::Cursor::Cell => "cell",
        servo::Cursor::Crosshair => "crosshair",
        servo::Cursor::Text => "text",
        servo::Cursor::VerticalText => "vertical-text",
        servo::Cursor::Alias => "alias",
        servo::Cursor::Copy => "copy",
        servo::Cursor::Move => "move",
        servo::Cursor::NoDrop => "no-drop",
        servo::Cursor::NotAllowed => "not-allowed",
        servo::Cursor::Grab => "grab",
        servo::Cursor::Grabbing => "grabbing",
        servo::Cursor::EResize => "e-resize",
        servo::Cursor::NResize => "n-resize",
        servo::Cursor::NeResize => "ne-resize",
        servo::Cursor::NwResize => "nw-resize",
        servo::Cursor::SResize => "s-resize",
        servo::Cursor::SeResize => "se-resize",
        servo::Cursor::SwResize => "sw-resize",
        servo::Cursor::WResize => "w-resize",
        servo::Cursor::EwResize => "ew-resize",
        servo::Cursor::NsResize => "ns-resize",
        servo::Cursor::NeswResize => "nesw-resize",
        servo::Cursor::NwseResize => "nwse-resize",
        servo::Cursor::ColResize => "col-resize",
        servo::Cursor::RowResize => "row-resize",
        servo::Cursor::AllScroll => "all-scroll",
        servo::Cursor::ZoomIn => "zoom-in",
        servo::Cursor::ZoomOut => "zoom-out",
    }
}

fn permission_feature_name(permission: servo::PermissionFeature) -> String {
    // Convert PermissionFeature to kebab-case web-facing permission name.
    // Uses Debug formatting with CamelCase->kebab-case normalization.
    // FUTURE: When Servo evolves PermissionFeature, consider switching to explicit match
    // arms with compile-time exhaustiveness checking, or a custom enum mapping layer.
    let raw = format!("{permission:?}");
    let mut normalized = String::with_capacity(raw.len() + 4);
    for (index, ch) in raw.chars().enumerate() {
        if ch.is_uppercase() {
            if index > 0 {
                normalized.push('-');
            }
            normalized.extend(ch.to_lowercase());
        } else {
            normalized.push(ch);
        }
    }
    normalized
}

fn permission_request_origin(webview: &WebView) -> Option<String> {
    let url = webview.url()?;
    let origin = url.origin().unicode_serialization();
    if origin == "null" {
        Some(url.to_string())
    } else {
        Some(origin)
    }
}

fn convert_select_element_option(
    option_or_optgroup: &servo::SelectElementOptionOrOptgroup,
) -> crate::SelectElementOptionOrOptgroup {
    use crate::{SelectElementOption, SelectElementOptionOrOptgroup};
    match option_or_optgroup {
        servo::SelectElementOptionOrOptgroup::Option(option) => {
            SelectElementOptionOrOptgroup::Option(SelectElementOption {
                id: option.id,
                label: option.label.clone(),
                is_disabled: option.is_disabled,
            })
        }
        servo::SelectElementOptionOrOptgroup::Optgroup { label, options } => {
            SelectElementOptionOrOptgroup::Optgroup {
                label: label.clone(),
                options: options
                    .iter()
                    .map(|opt| SelectElementOption {
                        id: opt.id,
                        label: opt.label.clone(),
                        is_disabled: opt.is_disabled,
                    })
                    .collect(),
            }
        }
    }
}

fn convert_context_menu_item(item: &servo::ContextMenuItem) -> ContextMenuItem {
    match item {
        servo::ContextMenuItem::Item {
            label,
            action,
            enabled,
        } => ContextMenuItem::Item {
            label: label.clone(),
            action: convert_servo_context_menu_action(*action),
            enabled: *enabled,
        },
        servo::ContextMenuItem::Separator => ContextMenuItem::Separator,
    }
}

fn convert_context_menu_element_info(
    element_info: &servo::ContextMenuElementInformation,
) -> ContextMenuElementInformation {
    let flags = element_info.flags;
    let context_type = if flags.contains(servo::ContextMenuElementInformationFlags::Link) {
        "link"
    } else if flags.contains(servo::ContextMenuElementInformationFlags::Image) {
        "image"
    } else if flags.contains(servo::ContextMenuElementInformationFlags::EditableText) {
        "input"
    } else if flags.contains(servo::ContextMenuElementInformationFlags::Selection) {
        "text"
    } else {
        "default"
    };
    ContextMenuElementInformation {
        is_link: flags.contains(servo::ContextMenuElementInformationFlags::Link),
        is_image: flags.contains(servo::ContextMenuElementInformationFlags::Image),
        is_editable_text: flags.contains(servo::ContextMenuElementInformationFlags::EditableText),
        has_selection: flags.contains(servo::ContextMenuElementInformationFlags::Selection),
        link_url: element_info.link_url.as_ref().map(ToString::to_string),
        image_url: element_info.image_url.as_ref().map(ToString::to_string),
        context_type: context_type.to_owned(),
    }
}

fn convert_servo_context_menu_action(action: servo::ContextMenuAction) -> ContextMenuAction {
    match action {
        servo::ContextMenuAction::GoBack => ContextMenuAction::GoBack,
        servo::ContextMenuAction::GoForward => ContextMenuAction::GoForward,
        servo::ContextMenuAction::Reload => ContextMenuAction::Reload,
        servo::ContextMenuAction::CopyLink => ContextMenuAction::CopyLink,
        servo::ContextMenuAction::OpenLinkInNewWebView => ContextMenuAction::OpenLinkInNewWebView,
        servo::ContextMenuAction::CopyImageLink => ContextMenuAction::CopyImageLink,
        servo::ContextMenuAction::OpenImageInNewView => ContextMenuAction::OpenImageInNewView,
        servo::ContextMenuAction::Cut => ContextMenuAction::Cut,
        servo::ContextMenuAction::Copy => ContextMenuAction::Copy,
        servo::ContextMenuAction::Paste => ContextMenuAction::Paste,
        servo::ContextMenuAction::SelectAll => ContextMenuAction::SelectAll,
    }
}

fn convert_context_menu_action(action: ContextMenuAction) -> servo::ContextMenuAction {
    match action {
        ContextMenuAction::GoBack => servo::ContextMenuAction::GoBack,
        ContextMenuAction::GoForward => servo::ContextMenuAction::GoForward,
        ContextMenuAction::Reload => servo::ContextMenuAction::Reload,
        ContextMenuAction::CopyLink => servo::ContextMenuAction::CopyLink,
        ContextMenuAction::OpenLinkInNewWebView => servo::ContextMenuAction::OpenLinkInNewWebView,
        ContextMenuAction::CopyImageLink => servo::ContextMenuAction::CopyImageLink,
        ContextMenuAction::OpenImageInNewView => servo::ContextMenuAction::OpenImageInNewView,
        ContextMenuAction::Cut => servo::ContextMenuAction::Cut,
        ContextMenuAction::Copy => servo::ContextMenuAction::Copy,
        ContextMenuAction::Paste => servo::ContextMenuAction::Paste,
        ContextMenuAction::SelectAll => servo::ContextMenuAction::SelectAll,
    }
}
