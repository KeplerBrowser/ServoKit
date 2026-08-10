use crate::native_window::release_native_window;
use crate::state::{ControlCommand, HostHandle};
use crate::{AndroidRenderBackend, NativeWindowHandle, ServoStatus, SurfaceSize};
#[cfg(test)]
use servokit_embedder::NavigationRequest;
use servokit_embedder::{HostEvent, JavaScriptEvaluationErrorKind, RuntimeError, WebViewCommand};
use servokit_host::HostSurface;

pub(crate) fn queue_backend_result(
    handle: &mut HostHandle,
    result: Result<Vec<HostEvent>, String>,
    diagnostic_url: Option<&str>,
) {
    match result {
        Ok(events) => handle.events.extend(events),
        Err(message) => handle.events.push_back(HostEvent::Error {
            url: diagnostic_url.map(str::to_owned),
            code: ServoStatus::BackendError as i32,
            message,
        }),
    }
}

pub(crate) fn push_runtime_error(
    handle: &mut HostHandle,
    error: RuntimeError,
    diagnostic_url: Option<&str>,
) {
    match error {
        RuntimeError::InvalidUrl(error) => handle.events.push_back(HostEvent::Error {
            url: diagnostic_url.map(str::to_owned),
            code: ServoStatus::InvalidUrl as i32,
            message: error.to_string(),
        }),
        RuntimeError::SurfaceAlreadyAttached(_)
        | RuntimeError::SurfaceNotAttached(_)
        | RuntimeError::ManagedChildSurfaceAlreadyAttached(_, _)
        | RuntimeError::ManagedChildSurfaceNotAttached(_, _) => {}
        RuntimeError::Host(error) => {
            handle.push_backend_error(diagnostic_url, error.message().to_owned());
        }
        RuntimeError::UnknownSession(_) | RuntimeError::UnknownWebView(_) => {
            handle.push_backend_error(diagnostic_url, error.to_string());
        }
    }
}

#[cfg(test)]
pub(crate) fn load_navigation_request(handle: &mut HostHandle, request: NavigationRequest) {
    let requested_url = request.url.clone();
    if let Err(error) = handle.dispatch_webview_command_runtime(WebViewCommand::LoadUrl(request)) {
        push_runtime_error(handle, error, Some(&requested_url));
    }
}

pub(crate) fn attach_surface_with_native_window(
    handle: &mut HostHandle,
    native_window: NativeWindowHandle,
    size: SurfaceSize,
    density: f32,
) {
    if handle.surface.is_some() || handle.native_window.is_some() {
        release_native_window(Some(native_window));
        return;
    }

    handle.pending_attach_native_window = Some(native_window);
    handle.pending_attach_density = Some(density);

    if let Err(error) =
        handle.attach_surface_runtime(HostSurface::new("android-native-window"), size)
    {
        release_native_window(handle.pending_attach_native_window.take());
        handle.pending_attach_density = None;
        let current_url = handle.current_url.clone();
        push_runtime_error(handle, error, current_url.as_deref());
    }
}

pub(crate) fn perform_android_navigation_command(
    handle: &mut HostHandle,
    command: impl FnOnce(&mut dyn AndroidRenderBackend) -> Result<Vec<HostEvent>, String>,
) {
    let result = {
        let Some(backend) = handle.android_backend.as_mut() else {
            return;
        };
        command(backend.as_mut())
    };
    let current_url = handle.current_url.clone();
    queue_backend_result(handle, result, current_url.as_deref());
}

fn queue_webview_not_ready_evaluation_result(handle: &mut HostHandle, evaluation_id: String) {
    handle
        .events
        .push_back(HostEvent::JavaScriptEvaluationResult {
            evaluation_id,
            ok: false,
            value_json: None,
            error_type: Some(JavaScriptEvaluationErrorKind::WebViewNotReady),
        });
}

fn perform_android_javascript_evaluation(
    handle: &mut HostHandle,
    evaluation_id: String,
    script: String,
) {
    let result = {
        let Some(backend) = handle.android_backend.as_mut() else {
            queue_webview_not_ready_evaluation_result(handle, evaluation_id);
            return;
        };
        backend.evaluate_javascript(&evaluation_id, &script)
    };
    let current_url = handle.current_url.clone();
    queue_backend_result(handle, result, current_url.as_deref());
}

pub(crate) fn queue_control_command(handle: &mut HostHandle, command: ControlCommand) {
    if matches!(command, ControlCommand::WebView(WebViewCommand::Reload))
        && handle.pending_commands.contains(&command)
    {
        return;
    }

    handle.pending_commands.push_back(command);
}

pub(crate) fn dispatch_webview_command(handle: &mut HostHandle, command: WebViewCommand) {
    let current_url = handle.current_url.clone();
    if let Err(error) = handle.dispatch_webview_command_runtime(command) {
        push_runtime_error(handle, error, current_url.as_deref());
    }
}

fn apply_control_command(handle: &mut HostHandle, command: ControlCommand) {
    match command {
        ControlCommand::WebView(command) => {
            dispatch_webview_command(handle, command);
        }
        ControlCommand::DispatchImeComposition { state, text } => {
            perform_android_navigation_command(handle, |backend| {
                backend.dispatch_ime_composition(state, &text)
            });
        }
        ControlCommand::DismissInputMethod => {
            perform_android_navigation_command(handle, |backend| backend.dismiss_input_method());
        }
        ControlCommand::DispatchKeyboardKey { key } => {
            perform_android_navigation_command(handle, |backend| {
                backend.dispatch_keyboard_key(key)
            });
        }
        ControlCommand::ResolveSimpleDialog {
            dialog_id,
            confirmed,
            prompt_value,
        } => {
            perform_android_navigation_command(handle, |backend| {
                backend.resolve_simple_dialog(&dialog_id, confirmed, prompt_value.as_deref())
            });
        }
        ControlCommand::ResolveSelectElement {
            select_element_id,
            selected_options,
        } => {
            perform_android_navigation_command(handle, |backend| {
                backend.resolve_select_element(&select_element_id, selected_options)
            });
        }
        ControlCommand::ResolveFilePicker {
            file_picker_id,
            selected_paths,
        } => {
            perform_android_navigation_command(handle, |backend| {
                backend.resolve_file_picker(&file_picker_id, selected_paths)
            });
        }
        ControlCommand::DismissFilePicker { file_picker_id } => {
            perform_android_navigation_command(handle, |backend| {
                backend.dismiss_file_picker(&file_picker_id)
            });
        }
        ControlCommand::ResolvePermission { allow } => {
            perform_android_navigation_command(handle, |backend| backend.resolve_permission(allow));
        }
        ControlCommand::ResolveNavigationRequest {
            navigation_id,
            allow,
        } => {
            perform_android_navigation_command(handle, |backend| {
                backend.resolve_navigation_request(&navigation_id, allow)
            });
        }
        ControlCommand::ResolveContextMenu {
            context_menu_id,
            action,
        } => {
            perform_android_navigation_command(handle, |backend| {
                backend.resolve_context_menu(&context_menu_id, action)
            });
        }
        ControlCommand::DismissContextMenu { context_menu_id } => {
            perform_android_navigation_command(handle, |backend| {
                backend.dismiss_context_menu(&context_menu_id)
            });
        }
        ControlCommand::EvaluateJavaScript {
            evaluation_id,
            script,
        } => {
            perform_android_javascript_evaluation(handle, evaluation_id, script);
        }
    }
}

pub(crate) fn perform_android_updates(handle: &mut HostHandle) {
    while let Some(command) = handle.pending_commands.pop_front() {
        apply_control_command(handle, command);
    }

    let current_url = handle.current_url.clone();
    if let Err(error) = handle.perform_updates_runtime() {
        push_runtime_error(handle, error, current_url.as_deref());
    }
}

pub(crate) fn load_navigation_request_from_str(
    handle: &mut HostHandle,
    url: &str,
) -> Result<(), ServoStatus> {
    match handle.load_url_runtime(url) {
        Ok(()) => Ok(()),
        Err(RuntimeError::InvalidUrl(error)) => {
            handle.events.push_back(HostEvent::Error {
                url: Some(url.to_owned()),
                code: ServoStatus::InvalidUrl as i32,
                message: error.to_string(),
            });
            Err(ServoStatus::InvalidUrl)
        }
        Err(error) => {
            push_runtime_error(handle, error, Some(url));
            Err(ServoStatus::BackendError)
        }
    }
}
