//! Four-function C boundary for the Servo-free portable controller.

use std::collections::HashMap;
use std::mem;
use std::num::NonZeroU64;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::{LazyLock, Mutex, MutexGuard};

use servokit_embedder::{PortableController, PortableControllerError, PortableControllerResult};

const MAX_INPUT_BYTES: usize = 1024 * 1024;

pub const SERVOKIT_CONTROLLER_OK: u32 = 0;
pub const SERVOKIT_CONTROLLER_INVALID_ARGUMENT: u32 = 1;
pub const SERVOKIT_CONTROLLER_STALE_HANDLE: u32 = 2;
pub const SERVOKIT_CONTROLLER_BUSY: u32 = 3;
pub const SERVOKIT_CONTROLLER_INTERNAL_ERROR: u32 = 4;
pub const SERVOKIT_CONTROLLER_PANIC: u32 = 5;

#[repr(C)]
#[derive(Debug)]
pub struct ServoKitControllerResult {
    pub status: u32,
    pub handle: u64,
    pub bytes: *const u8,
    pub len: usize,
}

struct Registry {
    next: Option<NonZeroU64>,
    controllers: HashMap<u64, Option<PortableController>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            next: NonZeroU64::new(1),
            controllers: HashMap::new(),
        }
    }
}

impl Registry {
    fn create(&mut self) -> Option<u64> {
        let handle = self.next?;
        self.next = handle.get().checked_add(1).and_then(NonZeroU64::new);
        self.controllers
            .insert(handle.get(), Some(PortableController::new(handle)));
        Some(handle.get())
    }

    fn take(&mut self, handle: u64) -> Result<PortableController, RegistryError> {
        let controller = self
            .controllers
            .get_mut(&handle)
            .ok_or(RegistryError::Stale)?;
        controller.take().ok_or(RegistryError::Busy)
    }

    fn restore(&mut self, handle: u64, controller: PortableController) {
        let slot = self
            .controllers
            .get_mut(&handle)
            .expect("accepted controller handle remains registered");
        assert!(
            slot.replace(controller).is_none(),
            "controller slot was not busy"
        );
    }

    fn take_for_destroy(&mut self, handle: u64) -> Result<PortableController, RegistryError> {
        let controller = self.take(handle)?;
        self.controllers.remove(&handle);
        Ok(controller)
    }
}

#[derive(Debug, Clone, Copy)]
enum RegistryError {
    Stale,
    Busy,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| Mutex::new(Registry::default()));

fn lock_registry() -> MutexGuard<'static, Registry> {
    REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn catch_boundary<T>(operation: impl FnOnce() -> T) -> Result<T, ()> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            // A custom payload may panic in Drop; leaking it is safer than unwinding through C.
            mem::forget(payload);
            Err(())
        }
    }
}

fn owned_result(status: u32, handle: u64, json: String) -> ServoKitControllerResult {
    let bytes = json.into_bytes().into_boxed_slice();
    let len = bytes.len();
    let bytes = Box::into_raw(bytes) as *mut u8 as *const u8;
    ServoKitControllerResult {
        status,
        handle,
        bytes,
        len,
    }
}

fn error_result(
    status: u32,
    handle: u64,
    code: &'static str,
    message: &'static str,
) -> ServoKitControllerResult {
    let code = serde_json_string(code);
    let message = serde_json_string(message);
    owned_result(
        status,
        handle,
        format!(
            r#"{{"version":1,"effects":[],"events":[],"error":{{"code":{code},"message":{message}}}}}"#
        ),
    )
}

fn serde_json_string(value: &str) -> String {
    serde_json::to_string(value).expect("strings are serializable as JSON")
}

fn registry_error(error: RegistryError, handle: u64) -> ServoKitControllerResult {
    match error {
        RegistryError::Stale => error_result(
            SERVOKIT_CONTROLLER_STALE_HANDLE,
            0,
            "staleHandle",
            "controller handle is not live",
        ),
        RegistryError::Busy => error_result(
            SERVOKIT_CONTROLLER_BUSY,
            handle,
            "busy",
            "controller handle is already in use",
        ),
    }
}

fn transport_error(
    handle: u64,
    controller: PortableController,
    message: &'static str,
) -> ServoKitControllerResult {
    lock_registry().restore(handle, controller);
    error_result(
        SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
        handle,
        "invalidArgument",
        message,
    )
}

fn reducer_error(
    handle: u64,
    controller: PortableController,
    error: PortableControllerError,
) -> ServoKitControllerResult {
    if error.is_internal() {
        lock_registry().controllers.remove(&handle);
        owned_result(
            SERVOKIT_CONTROLLER_INTERNAL_ERROR,
            0,
            error.to_result_json(),
        )
    } else {
        lock_registry().restore(handle, controller);
        owned_result(
            SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
            handle,
            error.to_result_json(),
        )
    }
}

#[cfg(test)]
thread_local! {
    static PANIC_NEXT_CREATE_AFTER_REGISTRATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INJECTED_CREATE_HANDLE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static PANIC_NEXT_DISPATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn inject_test_create_panic(_handle: u64) {
    #[cfg(test)]
    PANIC_NEXT_CREATE_AFTER_REGISTRATION.with(|panic| {
        if panic.replace(false) {
            INJECTED_CREATE_HANDLE.with(|injected| injected.set(_handle));
            panic!("injected create panic");
        }
    });
}

fn inject_test_dispatch_panic() {
    #[cfg(test)]
    PANIC_NEXT_DISPATCH.with(|panic| {
        if panic.replace(false) {
            panic!("injected dispatch panic");
        }
    });
}

/// Create a portable controller. The returned nonzero handle remains live until destroy or a
/// terminal dispatch failure.
#[no_mangle]
pub extern "C" fn servokit_controller_create() -> ServoKitControllerResult {
    let mut registered_handle = None;
    match catch_boundary(|| {
        let handle = match lock_registry().create() {
            Some(handle) => handle,
            None => {
                return error_result(
                    SERVOKIT_CONTROLLER_INTERNAL_ERROR,
                    0,
                    "internalError",
                    "controller handle space is exhausted",
                );
            }
        };
        registered_handle = Some(handle);
        inject_test_create_panic(handle);
        owned_result(
            SERVOKIT_CONTROLLER_OK,
            handle,
            PortableControllerResult::default().to_json(),
        )
    }) {
        Ok(result) => result,
        Err(()) => {
            if let Some(handle) = registered_handle {
                lock_registry().controllers.remove(&handle);
            }
            error_result(
                SERVOKIT_CONTROLLER_PANIC,
                0,
                "panic",
                "controller creation panicked",
            )
        }
    }
}

/// Validate and synchronously reduce one strict UTF-8 JSON v1 envelope.
///
/// # Safety
///
/// A non-null `bytes` must point to exactly `len` readable bytes for this call. Input is nonempty
/// and at most 1 MiB. Calls for one handle must be serialized.
#[no_mangle]
pub unsafe extern "C" fn servokit_controller_dispatch(
    handle: u64,
    bytes: *const u8,
    len: usize,
) -> ServoKitControllerResult {
    let mut accepted = false;
    match catch_boundary(|| {
        let mut controller = match lock_registry().take(handle) {
            Ok(controller) => controller,
            Err(error) => return registry_error(error, handle),
        };
        accepted = true;
        if bytes.is_null() || len == 0 || len > MAX_INPUT_BYTES {
            return transport_error(
                handle,
                controller,
                "input must be nonempty and at most 1 MiB",
            );
        }
        let bytes = unsafe { std::slice::from_raw_parts(bytes, len) };
        let input = match std::str::from_utf8(bytes) {
            Ok(input) => input,
            Err(_) => return transport_error(handle, controller, "input must be strict UTF-8"),
        };
        inject_test_dispatch_panic();
        match controller.dispatch_json(input) {
            Ok(result) => {
                lock_registry().restore(handle, controller);
                owned_result(SERVOKIT_CONTROLLER_OK, handle, result.to_json())
            }
            Err(error) => reducer_error(handle, controller, error),
        }
    }) {
        Ok(result) => result,
        Err(()) => {
            if accepted {
                lock_registry().controllers.remove(&handle);
            }
            error_result(
                SERVOKIT_CONTROLLER_PANIC,
                0,
                "panic",
                "controller dispatch panicked",
            )
        }
    }
}

/// Retire a live handle and return terminal effects/events for all pending controls.
#[no_mangle]
pub extern "C" fn servokit_controller_destroy(handle: u64) -> ServoKitControllerResult {
    let mut accepted = false;
    match catch_boundary(|| {
        let controller = match lock_registry().take_for_destroy(handle) {
            Ok(controller) => controller,
            Err(error) => return registry_error(error, handle),
        };
        accepted = true;
        owned_result(SERVOKIT_CONTROLLER_OK, 0, controller.destroy().to_json())
    }) {
        Ok(result) => result,
        Err(()) => {
            if accepted {
                lock_registry().controllers.remove(&handle);
            }
            error_result(
                SERVOKIT_CONTROLLER_PANIC,
                0,
                "panic",
                "controller destroy panicked",
            )
        }
    }
}

/// Free one exact-length owned result allocation. Null/zero empty results are accepted.
///
/// # Safety
///
/// A nonempty result's `bytes` and `len` must be the unchanged allocation fields returned by one
/// ServoKit controller ABI call. That allocation must not have been freed before, and neither its
/// pointer nor its bytes may be used after this call. Passing a mutated result or freeing one
/// result more than once is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn servokit_controller_result_free(result: ServoKitControllerResult) {
    let _ = catch_boundary(|| {
        if result.bytes.is_null() || result.len == 0 {
            return;
        }
        let bytes = ptr::slice_from_raw_parts_mut(result.bytes as *mut u8, result.len);
        drop(unsafe { Box::from_raw(bytes) });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn create() -> (u64, Value) {
        let result = servokit_controller_create();
        let handle = result.handle;
        let (status, echoed, json) = copy_and_free(result);
        assert_eq!(status, SERVOKIT_CONTROLLER_OK);
        assert_eq!(echoed, handle);
        (handle, json)
    }

    fn dispatch(handle: u64, input: &[u8]) -> (u32, u64, Value) {
        let result = unsafe { servokit_controller_dispatch(handle, input.as_ptr(), input.len()) };
        copy_and_free(result)
    }

    fn copy_and_free(result: ServoKitControllerResult) -> (u32, u64, Value) {
        assert!(!result.bytes.is_null());
        assert!(result.len > 0);
        let bytes = unsafe { std::slice::from_raw_parts(result.bytes, result.len) };
        assert_ne!(bytes.last(), Some(&0));
        let json = serde_json::from_slice(bytes).unwrap();
        let status = result.status;
        let handle = result.handle;
        unsafe { servokit_controller_result_free(result) };
        (status, handle, json)
    }

    fn assert_transport_precedence(handle: u64, status: u32, echoed_handle: u64) {
        let nonempty = [b'x'];
        let oversized = vec![b' '; MAX_INPUT_BYTES + 1];
        let invalid_utf8 = [0xff];
        let results = [
            unsafe { servokit_controller_dispatch(handle, ptr::null(), 1) },
            unsafe { servokit_controller_dispatch(handle, nonempty.as_ptr(), 0) },
            unsafe { servokit_controller_dispatch(handle, oversized.as_ptr(), oversized.len()) },
            unsafe {
                servokit_controller_dispatch(handle, invalid_utf8.as_ptr(), invalid_utf8.len())
            },
        ];
        for result in results {
            let (actual_status, actual_handle, json) = copy_and_free(result);
            assert_eq!((actual_status, actual_handle), (status, echoed_handle));
            assert_eq!(json["effects"], serde_json::json!([]));
        }
    }

    #[test]
    fn creates_reloads_destroys_and_then_reports_stale() {
        let (handle, created) = create();
        assert_ne!(handle, 0);
        assert_eq!(created["version"], 1);
        let (status, echoed, reloaded) = dispatch(handle, br#"{"version":1,"command":"reload"}"#);
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_OK, handle));
        assert_eq!(reloaded["effects"][0]["name"], "reload");

        let (status, echoed, destroyed) = copy_and_free(servokit_controller_destroy(handle));
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_OK, 0));
        assert_eq!(destroyed["effects"][0]["name"], "destroy");
        let (status, echoed, stale) = dispatch(handle, br#"{"version":1,"command":"reload"}"#);
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_STALE_HANDLE, 0));
        assert_eq!(stale["effects"], serde_json::json!([]));
    }

    #[test]
    fn invalid_inputs_are_nonterminal_and_have_no_effects() {
        let (handle, _) = create();
        let invalid = [
            br#"{"command":"reload"}"#.as_slice(),
            br#"{"version":1,"command":"reload","observation":"urlChanged"}"#.as_slice(),
            b"\xff".as_slice(),
        ];
        for input in invalid {
            let (status, echoed, result) = dispatch(handle, input);
            assert_eq!(
                (status, echoed),
                (SERVOKIT_CONTROLLER_INVALID_ARGUMENT, handle)
            );
            assert_eq!(result["effects"], serde_json::json!([]));
            assert!(result.get("error").is_some());
        }
        let empty = unsafe { servokit_controller_dispatch(handle, ptr::null(), 0) };
        let (status, echoed, _) = copy_and_free(empty);
        assert_eq!(
            (status, echoed),
            (SERVOKIT_CONTROLLER_INVALID_ARGUMENT, handle)
        );
        let oversized = vec![b' '; MAX_INPUT_BYTES + 1];
        let (status, echoed, _) = dispatch(handle, &oversized);
        assert_eq!(
            (status, echoed),
            (SERVOKIT_CONTROLLER_INVALID_ARGUMENT, handle)
        );
        let exact_limit = vec![b' '; MAX_INPUT_BYTES];
        let (status, echoed, result) = dispatch(handle, &exact_limit);
        assert_eq!(
            (status, echoed),
            (SERVOKIT_CONTROLLER_INVALID_ARGUMENT, handle)
        );
        assert_eq!(result["error"]["code"], "invalidEnvelope");
        let (status, echoed, result) = dispatch(handle, br#"{"version":1,"command":"reload"}"#);
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_OK, handle));
        assert_eq!(result["effects"][0]["name"], "reload");
        copy_and_free(servokit_controller_destroy(handle));
    }

    #[test]
    fn owned_results_are_exact_slices_and_null_empty_free_is_allowed() {
        let (handle, _) = create();
        let result = unsafe {
            servokit_controller_dispatch(
                handle,
                br#"{"version":1,"command":"reload"}"#.as_ptr(),
                br#"{"version":1,"command":"reload"}"#.len(),
            )
        };
        let bytes = unsafe { std::slice::from_raw_parts(result.bytes, result.len) };
        assert_eq!(bytes.first(), Some(&b'{'));
        assert_eq!(bytes.last(), Some(&b'}'));
        unsafe {
            servokit_controller_result_free(result);
            servokit_controller_result_free(ServoKitControllerResult {
                status: SERVOKIT_CONTROLLER_OK,
                handle: 0,
                bytes: ptr::null(),
                len: 0,
            });
        }
        copy_and_free(servokit_controller_destroy(handle));
    }

    #[test]
    fn stale_and_busy_handles_take_precedence_over_invalid_transport() {
        let (handle, _) = create();
        let controller = lock_registry().take(handle).unwrap();
        assert_transport_precedence(handle, SERVOKIT_CONTROLLER_BUSY, handle);
        lock_registry().restore(handle, controller);
        copy_and_free(servokit_controller_destroy(handle));
        assert_transport_precedence(handle, SERVOKIT_CONTROLLER_STALE_HANDLE, 0);
    }

    #[test]
    fn busy_and_cross_thread_calls_follow_registry_rules() {
        let (busy_handle, _) = create();
        let controller = lock_registry().take(busy_handle).unwrap();
        let (status, echoed, result) =
            dispatch(busy_handle, br#"{"version":1,"command":"reload"}"#);
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_BUSY, busy_handle));
        assert_eq!(result["effects"], serde_json::json!([]));
        lock_registry().restore(busy_handle, controller);
        copy_and_free(servokit_controller_destroy(busy_handle));

        let (thread_handle, _) = create();
        let (status, echoed) = std::thread::spawn(move || {
            let (status, echoed, _) =
                dispatch(thread_handle, br#"{"version":1,"command":"reload"}"#);
            (status, echoed)
        })
        .join()
        .unwrap();
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_OK, thread_handle));
        copy_and_free(servokit_controller_destroy(thread_handle));
    }

    #[test]
    fn accepted_panic_is_contained_and_retires_handle() {
        let (handle, _) = create();
        PANIC_NEXT_DISPATCH.with(|panic| panic.set(true));
        let (status, echoed, result) = dispatch(handle, br#"{"version":1,"command":"reload"}"#);
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_PANIC, 0));
        assert_eq!(result["effects"], serde_json::json!([]));
        let (status, echoed, _) = dispatch(handle, br#"{"version":1,"command":"reload"}"#);
        assert_eq!((status, echoed), (SERVOKIT_CONTROLLER_STALE_HANDLE, 0));
    }

    #[test]
    fn create_panic_after_registration_does_not_strand_handle() {
        PANIC_NEXT_CREATE_AFTER_REGISTRATION.with(|panic| panic.set(true));
        let (status, handle, result) = copy_and_free(servokit_controller_create());
        assert_eq!((status, handle), (SERVOKIT_CONTROLLER_PANIC, 0));
        assert_eq!(result["effects"], serde_json::json!([]));

        let injected = INJECTED_CREATE_HANDLE.with(std::cell::Cell::get);
        assert_ne!(injected, 0);
        assert!(!lock_registry().controllers.contains_key(&injected));
    }

    #[test]
    fn c_layout_status_values_are_locked() {
        assert_eq!(SERVOKIT_CONTROLLER_OK, 0);
        assert_eq!(SERVOKIT_CONTROLLER_INVALID_ARGUMENT, 1);
        assert_eq!(SERVOKIT_CONTROLLER_STALE_HANDLE, 2);
        assert_eq!(SERVOKIT_CONTROLLER_BUSY, 3);
        assert_eq!(SERVOKIT_CONTROLLER_INTERNAL_ERROR, 4);
        assert_eq!(SERVOKIT_CONTROLLER_PANIC, 5);
        assert_eq!(std::mem::size_of::<ServoKitControllerResult>(), 32);
        assert_eq!(std::mem::align_of::<ServoKitControllerResult>(), 8);
    }
}
