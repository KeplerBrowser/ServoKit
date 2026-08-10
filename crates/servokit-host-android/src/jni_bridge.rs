use std::ffi::{c_char, CString};
use std::sync::OnceLock;

use jni::objects::{GlobalRef, JClass, JIntArray, JObject, JObjectArray, JString, JValue};
use jni::sys::{jfloat, jint, jintArray, jlong, jobjectArray};
use jni::{JNIEnv, JavaVM};

use crate::{
    servo_host_attach_surface_with_native_window, servo_host_blur,
    servo_host_controller_handle_token, servo_host_detach_surface, servo_host_dismiss_context_menu,
    servo_host_dismiss_context_menu_with_token_ffi, servo_host_dismiss_file_picker,
    servo_host_dismiss_input_method, servo_host_dispatch_ime_composition,
    servo_host_dispatch_keyboard_key, servo_host_dispatch_touch_event, servo_host_focus,
    servo_host_free, servo_host_go_back, servo_host_go_forward,
    servo_host_has_pending_permission_request, servo_host_last_event_bridge_json_copy,
    servo_host_load_url, servo_host_new, servo_host_pending_permission_request_origin_copy,
    servo_host_pending_permission_request_origin_len,
    servo_host_pending_permission_request_permission_copy,
    servo_host_pending_permission_request_permission_len, servo_host_perform_updates,
    servo_host_reload, servo_host_resize_surface, servo_host_resolve_context_menu,
    servo_host_resolve_context_menu_with_token_ffi, servo_host_resolve_file_picker,
    servo_host_resolve_navigation_request, servo_host_resolve_permission,
    servo_host_resolve_select_element, servo_host_resolve_simple_dialog,
    servo_host_send_controller_command_with_token_ffi, servo_host_take_next_event_bridge_json_len,
    servo_host_trigger_context_menu, HostHandle, ServoStatus,
};

unsafe extern "C" {
    fn ANativeWindow_fromSurface(
        env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
    ) -> *mut std::ffi::c_void;
}

struct AndroidJvmBridge {
    java_vm: JavaVM,
    jni_servo_host_class: GlobalRef,
}

static ANDROID_JVM_BRIDGE: OnceLock<AndroidJvmBridge> = OnceLock::new();

fn initialize_android_jvm(env: &mut JNIEnv<'_>, class: JClass<'_>) {
    let Ok(java_vm) = env.get_java_vm() else {
        return;
    };
    let Ok(jni_servo_host_class) = env.new_global_ref(class) else {
        return;
    };
    let _ = ANDROID_JVM_BRIDGE.set(AndroidJvmBridge {
        java_vm,
        jni_servo_host_class,
    });
}

fn with_android_jvm<T>(
    callback: impl FnOnce(&mut JNIEnv<'_>, &GlobalRef) -> Result<T, String>,
) -> Result<T, String> {
    let bridge = ANDROID_JVM_BRIDGE
        .get()
        .ok_or_else(|| "android JVM bridge not initialized".to_owned())?;
    let mut env = bridge
        .java_vm
        .attach_current_thread_permanently()
        .map_err(|error| error.to_string())?;
    callback(&mut env, &bridge.jni_servo_host_class)
}

pub(crate) fn android_platform_clipboard_get_text() -> Result<Option<String>, String> {
    with_android_jvm(|env, class| {
        let value = env
            .call_static_method(
                class,
                "platformGetClipboardText",
                "()Ljava/lang/String;",
                &[],
            )
            .map_err(|error| error.to_string())?
            .l()
            .map_err(|error| error.to_string())?;
        if value.is_null() {
            return Ok(None);
        }

        let value = JString::from(value);
        let value = env.get_string(&value).map_err(|error| error.to_string())?;
        Ok(Some(value.to_string_lossy().into_owned()))
    })
}

pub(crate) fn android_platform_clipboard_set_text(text: &str) -> Result<bool, String> {
    with_android_jvm(|env, class| {
        let text = env.new_string(text).map_err(|error| error.to_string())?;
        env.call_static_method(
            class,
            "platformSetClipboardText",
            "(Ljava/lang/String;)Z",
            &[JValue::Object(&JObject::from(text))],
        )
        .map_err(|error| error.to_string())?
        .z()
        .map_err(|error| error.to_string())
    })
}

pub(crate) fn android_platform_clipboard_clear_text() -> Result<bool, String> {
    with_android_jvm(|env, class| {
        env.call_static_method(class, "platformClearClipboardText", "()Z", &[])
            .map_err(|error| error.to_string())?
            .z()
            .map_err(|error| error.to_string())
    })
}

fn handle_from_jlong(handle: jlong) -> *mut HostHandle {
    handle as *mut HostHandle
}

fn copy_string(length: usize, copy_fn: impl FnOnce(*mut c_char, usize) -> usize) -> String {
    if length == 0 {
        return String::new();
    }

    let mut buffer = vec![0u8; length + 1];
    let written = copy_fn(buffer.as_mut_ptr().cast(), buffer.len());
    String::from_utf8_lossy(&buffer[..written]).into_owned()
}

fn new_jstring(env: &mut JNIEnv<'_>, value: String) -> jni::sys::jstring {
    env.new_string(value)
        .map(|value| value.into_raw())
        .unwrap_or_else(|_| JObject::null().into_raw())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeCreateHost(
    mut env: JNIEnv<'_>,
    class: JClass<'_>,
) -> jlong {
    initialize_android_jvm(&mut env, class);
    servo_host_new(0) as jlong
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeLoadUrl(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    input: JString<'_>,
) -> jint {
    if handle == 0 {
        return ServoStatus::NullPointer as jint;
    }

    let Ok(input) = env.get_string(&input) else {
        return ServoStatus::NullPointer as jint;
    };
    let Ok(input) = CString::new(input.to_string_lossy().into_owned()) else {
        return ServoStatus::InvalidUrl as jint;
    };

    unsafe { servo_host_load_url(handle_from_jlong(handle), input.as_ptr()) as jint }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeAttachSurface(
    env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    surface: JObject<'_>,
    width: jint,
    height: jint,
    density: jfloat,
) {
    if handle == 0 || surface.is_null() {
        return;
    }

    let native_window =
        unsafe { ANativeWindow_fromSurface(env.get_native_interface(), surface.into_raw()) };
    if native_window.is_null() {
        return;
    }

    unsafe {
        servo_host_attach_surface_with_native_window(
            handle_from_jlong(handle),
            native_window,
            width as u32,
            height as u32,
            density,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResizeSurface(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    width: jint,
    height: jint,
    density: jfloat,
) {
    if handle == 0 {
        return;
    }

    unsafe {
        servo_host_resize_surface(
            handle_from_jlong(handle),
            width as u32,
            height as u32,
            density,
        )
    };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDetachSurface(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_detach_surface(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativePerformUpdates(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_perform_updates(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDispatchTouchEvent(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    action: jint,
    touch_id: jint,
    x: jfloat,
    y: jfloat,
) {
    if handle == 0 {
        return;
    }

    unsafe {
        servo_host_dispatch_touch_event(handle_from_jlong(handle), action, touch_id, x, y);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeTriggerContextMenu(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    x: jfloat,
    y: jfloat,
) {
    if handle == 0 {
        return;
    }

    unsafe {
        servo_host_trigger_context_menu(handle_from_jlong(handle), x, y);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDispatchImeComposition(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    state: jint,
    text: JString<'_>,
) -> jint {
    if handle == 0 {
        return ServoStatus::NullPointer as jint;
    }

    let Ok(text) = env.get_string(&text) else {
        return ServoStatus::NullPointer as jint;
    };
    let Ok(text) = CString::new(text.to_string_lossy().into_owned()) else {
        return ServoStatus::BackendError as jint;
    };

    unsafe {
        servo_host_dispatch_ime_composition(handle_from_jlong(handle), state, text.as_ptr()) as jint
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDismissInputMethod(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) -> jint {
    if handle == 0 {
        return ServoStatus::NullPointer as jint;
    }

    unsafe { servo_host_dismiss_input_method(handle_from_jlong(handle)) as jint }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDispatchKeyboardKey(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    key: jint,
) -> jint {
    if handle == 0 {
        return ServoStatus::NullPointer as jint;
    }

    unsafe { servo_host_dispatch_keyboard_key(handle_from_jlong(handle), key) as jint }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolveSimpleDialog(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    dialog_id: JString<'_>,
    action: jint,
    prompt_value: JObject<'_>,
) {
    if handle == 0 {
        return;
    }

    let Ok(dialog_id) = env.get_string(&dialog_id) else {
        return;
    };
    let Ok(dialog_id) = CString::new(dialog_id.to_string_lossy().into_owned()) else {
        return;
    };

    let prompt_value = if prompt_value.is_null() {
        None
    } else {
        let prompt_value = JString::from(prompt_value);
        let Ok(prompt_value) = env.get_string(&prompt_value) else {
            return;
        };
        CString::new(prompt_value.to_string_lossy().into_owned()).ok()
    };

    unsafe {
        servo_host_resolve_simple_dialog(
            handle_from_jlong(handle),
            dialog_id.as_ptr(),
            action,
            prompt_value
                .as_ref()
                .map_or(std::ptr::null(), |value| value.as_ptr()),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolveSelectElement(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    select_element_id: JString<'_>,
    selected_options: jintArray,
) {
    if handle == 0 {
        return;
    }

    let Ok(select_element_id) = env.get_string(&select_element_id) else {
        return;
    };
    let Ok(select_element_id) = CString::new(select_element_id.to_string_lossy().into_owned())
    else {
        return;
    };

    let selected_options = unsafe { JIntArray::from_raw(selected_options) };
    let Ok(selected_options_len) = env.get_array_length(&selected_options) else {
        return;
    };
    let mut selected_options_array = vec![0; selected_options_len as usize];
    if env
        .get_int_array_region(&selected_options, 0, &mut selected_options_array)
        .is_err()
    {
        return;
    }
    let selected_options = selected_options_array
        .iter()
        .map(|value| *value as usize)
        .collect::<Vec<_>>();

    unsafe {
        servo_host_resolve_select_element(
            handle_from_jlong(handle),
            select_element_id.as_ptr(),
            selected_options.as_ptr(),
            selected_options.len(),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolveFilePicker(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    file_picker_id: JString<'_>,
    selected_paths: jobjectArray,
) {
    if handle == 0 {
        return;
    }

    let Ok(file_picker_id) = env.get_string(&file_picker_id) else {
        return;
    };
    let Ok(file_picker_id) = CString::new(file_picker_id.to_string_lossy().into_owned()) else {
        return;
    };

    let selected_paths = unsafe { JObjectArray::from_raw(selected_paths) };
    let Ok(selected_paths_len) = env.get_array_length(&selected_paths) else {
        return;
    };

    let mut selected_path_strings = Vec::with_capacity(selected_paths_len as usize);
    for index in 0..selected_paths_len {
        let Ok(selected_path) = env.get_object_array_element(&selected_paths, index) else {
            return;
        };
        let selected_path = JString::from(selected_path);
        let Ok(selected_path) = env.get_string(&selected_path) else {
            return;
        };
        let Ok(selected_path) = CString::new(selected_path.to_string_lossy().into_owned()) else {
            return;
        };
        selected_path_strings.push(selected_path);
    }

    let selected_path_ptrs = selected_path_strings
        .iter()
        .map(|selected_path| selected_path.as_ptr())
        .collect::<Vec<_>>();

    unsafe {
        servo_host_resolve_file_picker(
            handle_from_jlong(handle),
            file_picker_id.as_ptr(),
            selected_path_ptrs.as_ptr(),
            selected_path_ptrs.len(),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDismissFilePicker(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    file_picker_id: JString<'_>,
) {
    if handle == 0 {
        return;
    }

    let Ok(file_picker_id) = env.get_string(&file_picker_id) else {
        return;
    };
    let Ok(file_picker_id) = CString::new(file_picker_id.to_string_lossy().into_owned()) else {
        return;
    };

    unsafe {
        servo_host_dismiss_file_picker(handle_from_jlong(handle), file_picker_id.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeHasPendingPermissionRequest(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) -> bool {
    if handle == 0 {
        return false;
    }

    unsafe { servo_host_has_pending_permission_request(handle_from_jlong(handle)) }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativePendingPermissionRequestPermission(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) -> jni::sys::jstring {
    if handle == 0 {
        return JObject::null().into_raw();
    }

    let length =
        unsafe { servo_host_pending_permission_request_permission_len(handle_from_jlong(handle)) };
    let value = copy_string(length, |output, capacity| unsafe {
        servo_host_pending_permission_request_permission_copy(
            handle_from_jlong(handle),
            output,
            capacity,
        )
    });
    new_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativePendingPermissionRequestOrigin(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) -> jni::sys::jstring {
    if handle == 0 {
        return JObject::null().into_raw();
    }

    let length =
        unsafe { servo_host_pending_permission_request_origin_len(handle_from_jlong(handle)) };
    let value = copy_string(length, |output, capacity| unsafe {
        servo_host_pending_permission_request_origin_copy(
            handle_from_jlong(handle),
            output,
            capacity,
        )
    });
    new_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolvePermission(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    allow: bool,
) -> jint {
    if handle == 0 {
        return ServoStatus::NullPointer as jint;
    }

    unsafe { servo_host_resolve_permission(handle_from_jlong(handle), allow) as jint }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolveNavigationRequest(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    navigation_id: JString<'_>,
    allow: bool,
) -> jint {
    if handle == 0 {
        return ServoStatus::NullPointer as jint;
    }

    let Ok(navigation_id) = env.get_string(&navigation_id) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(navigation_id) = CString::new(navigation_id.to_string_lossy().into_owned()) else {
        return ServoStatus::BackendError as jint;
    };

    unsafe {
        servo_host_resolve_navigation_request(
            handle_from_jlong(handle),
            navigation_id.as_ptr(),
            allow,
        ) as jint
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolveContextMenu(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    context_menu_id: JString<'_>,
    action: JString<'_>,
) {
    if handle == 0 {
        return;
    }

    let Ok(context_menu_id) = env.get_string(&context_menu_id) else {
        return;
    };
    let Ok(context_menu_id) = CString::new(context_menu_id.to_string_lossy().into_owned()) else {
        return;
    };
    let Ok(action) = env.get_string(&action) else {
        return;
    };
    let Ok(action) = CString::new(action.to_string_lossy().into_owned()) else {
        return;
    };

    unsafe {
        servo_host_resolve_context_menu(
            handle_from_jlong(handle),
            context_menu_id.as_ptr(),
            action.as_ptr(),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeResolveContextMenuWithController(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    controller_handle: JString<'_>,
    context_menu_id: JString<'_>,
    action: JString<'_>,
) -> jint {
    let Ok(controller_handle) = env.get_string(&controller_handle) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(controller_handle) = CString::new(controller_handle.to_string_lossy().into_owned())
    else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(context_menu_id) = env.get_string(&context_menu_id) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(context_menu_id) = CString::new(context_menu_id.to_string_lossy().into_owned()) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(action) = env.get_string(&action) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(action) = CString::new(action.to_string_lossy().into_owned()) else {
        return ServoStatus::BackendError as jint;
    };

    unsafe {
        servo_host_resolve_context_menu_with_token_ffi(
            controller_handle.as_ptr(),
            context_menu_id.as_ptr(),
            action.as_ptr(),
        ) as jint
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeSendControllerCommandWithController(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    controller_handle: JString<'_>,
    command_json: JString<'_>,
) -> jint {
    let Ok(controller_handle) = env.get_string(&controller_handle) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(controller_handle) = CString::new(controller_handle.to_string_lossy().into_owned())
    else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(command_json) = env.get_string(&command_json) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(command_json) = CString::new(command_json.to_string_lossy().into_owned()) else {
        return ServoStatus::BackendError as jint;
    };

    unsafe {
        servo_host_send_controller_command_with_token_ffi(
            controller_handle.as_ptr(),
            command_json.as_ptr(),
        ) as jint
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDismissContextMenu(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
    context_menu_id: JString<'_>,
) {
    if handle == 0 {
        return;
    }

    let Ok(context_menu_id) = env.get_string(&context_menu_id) else {
        return;
    };
    let Ok(context_menu_id) = CString::new(context_menu_id.to_string_lossy().into_owned()) else {
        return;
    };

    unsafe {
        servo_host_dismiss_context_menu(handle_from_jlong(handle), context_menu_id.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDismissContextMenuWithController(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    controller_handle: JString<'_>,
    context_menu_id: JString<'_>,
) -> jint {
    let Ok(controller_handle) = env.get_string(&controller_handle) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(controller_handle) = CString::new(controller_handle.to_string_lossy().into_owned())
    else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(context_menu_id) = env.get_string(&context_menu_id) else {
        return ServoStatus::BackendError as jint;
    };
    let Ok(context_menu_id) = CString::new(context_menu_id.to_string_lossy().into_owned()) else {
        return ServoStatus::BackendError as jint;
    };

    unsafe {
        servo_host_dismiss_context_menu_with_token_ffi(
            controller_handle.as_ptr(),
            context_menu_id.as_ptr(),
        ) as jint
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeTakeNextEventBridgeJson(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) -> jni::sys::jstring {
    if handle == 0 {
        return JObject::null().into_raw();
    }

    let length = unsafe { servo_host_take_next_event_bridge_json_len(handle_from_jlong(handle)) };
    if length == 0 {
        return JObject::null().into_raw();
    }

    let value = copy_string(length, |output, capacity| unsafe {
        servo_host_last_event_bridge_json_copy(handle_from_jlong(handle), output, capacity)
    });
    new_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeControllerHandle(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) -> jni::sys::jstring {
    let value = servo_host_controller_handle_token(handle_from_jlong(handle)).unwrap_or_default();
    new_jstring(&mut env, value)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeReload(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_reload(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeGoBack(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_go_back(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeGoForward(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_go_forward(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeFocus(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_focus(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeBlur(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    unsafe { servo_host_blur(handle_from_jlong(handle)) };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_servo_servokit_androidhost_JniServoHost_nativeDestroyHost(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    handle: jlong,
) {
    unsafe { servo_host_free(handle_from_jlong(handle)) };
}
