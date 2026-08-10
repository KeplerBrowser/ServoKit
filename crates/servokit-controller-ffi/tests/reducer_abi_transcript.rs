use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};
use servokit_controller_ffi::{
    servokit_controller_create, servokit_controller_destroy, servokit_controller_dispatch,
    servokit_controller_result_free, ServoKitControllerResult,
    SERVOKIT_CONTROLLER_INVALID_ARGUMENT, SERVOKIT_CONTROLLER_OK, SERVOKIT_CONTROLLER_STALE_HANDLE,
};

struct AbiResult {
    status: u32,
    handle: u64,
    json: Value,
}

fn copy_and_free(result: ServoKitControllerResult) -> AbiResult {
    assert!(!result.bytes.is_null());
    assert!(result.len > 0);
    let bytes = unsafe { std::slice::from_raw_parts(result.bytes, result.len) };
    assert_ne!(bytes.last(), Some(&0));
    let json = serde_json::from_slice(bytes).expect("ABI result must be JSON");
    let copied = AbiResult {
        status: result.status,
        handle: result.handle,
        json,
    };
    unsafe { servokit_controller_result_free(result) };
    copied
}

fn record_create(steps: &mut Vec<Value>, expected_json: Value) -> u64 {
    let result = copy_and_free(servokit_controller_create());
    assert_eq!(result.status, SERVOKIT_CONTROLLER_OK);
    assert_eq!(
        result.handle, 1,
        "integration test must run in a fresh process"
    );
    assert_eq!(result.json, expected_json);
    steps.push(json!({
        "label": "create",
        "operation": "servokit_controller_create",
        "result": {
            "status": result.status,
            "handle": result.handle,
            "json": result.json,
        },
    }));
    result.handle
}

fn record_dispatch(
    steps: &mut Vec<Value>,
    label: &str,
    handle: u64,
    input: Value,
    expected_status: u32,
    expected_handle: u64,
    expected_json: Value,
) {
    let bytes = serde_json::to_vec(&input).expect("input fixture must serialize");
    let result =
        copy_and_free(unsafe { servokit_controller_dispatch(handle, bytes.as_ptr(), bytes.len()) });
    assert_eq!(result.status, expected_status, "{label} status");
    assert_eq!(result.handle, expected_handle, "{label} handle");
    assert_eq!(result.json, expected_json, "{label} JSON");
    steps.push(json!({
        "label": label,
        "operation": "servokit_controller_dispatch",
        "handle": handle,
        "input": input,
        "result": {
            "status": result.status,
            "handle": result.handle,
            "json": result.json,
        },
    }));
}

fn record_destroy(
    steps: &mut Vec<Value>,
    handle: u64,
    expected_status: u32,
    expected_handle: u64,
    expected_json: Value,
) {
    let result = copy_and_free(servokit_controller_destroy(handle));
    assert_eq!(result.status, expected_status, "destroy status");
    assert_eq!(result.handle, expected_handle, "destroy handle");
    assert_eq!(result.json, expected_json, "destroy JSON");
    steps.push(json!({
        "label": "destroy",
        "operation": "servokit_controller_destroy",
        "handle": handle,
        "result": {
            "status": result.status,
            "handle": result.handle,
            "json": result.json,
        },
    }));
}

#[test]
fn reducer_abi_transcript() {
    let mut steps = Vec::new();
    let handle = record_create(
        &mut steps,
        json!({"version": 1, "effects": [], "events": []}),
    );

    record_dispatch(
        &mut steps,
        "reload",
        handle,
        json!({"version": 1, "command": "reload"}),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{"name": "reload"}],
            "events": [],
        }),
    );
    record_dispatch(
        &mut steps,
        "invalid-envelope",
        handle,
        json!({"command": "reload"}),
        SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
        handle,
        json!({
            "version": 1,
            "effects": [],
            "events": [],
            "error": {
                "code": "invalidEnvelope",
                "message": "version must be the integer 1",
            },
        }),
    );
    record_dispatch(
        &mut steps,
        "navigation-request",
        handle,
        json!({
            "version": 1,
            "observation": "navigationRequest",
            "url": "https://one.invalid/",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainNavigationCompletion",
                    "navigationId": "navigation-1-1-1",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "navigation-1-1-1",
                    "kind": "navigation",
                    "timeoutMs": 5000,
                },
            ],
            "events": [{
                "name": "navigationRequested",
                "payload": {
                    "navigationId": "navigation-1-1-1",
                    "url": "https://one.invalid/",
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "navigation-response",
        handle,
        json!({
            "version": 1,
            "command": "resolveNavigationRequest",
            "navigationId": "navigation-1-1-1",
            "allow": false,
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{
                "name": "resolveNavigationCompletion",
                "navigationId": "navigation-1-1-1",
                "allow": false,
            }],
            "events": [],
        }),
    );
    record_dispatch(
        &mut steps,
        "duplicate-navigation-response",
        handle,
        json!({
            "version": 1,
            "command": "resolveNavigationRequest",
            "navigationId": "navigation-1-1-1",
            "allow": true,
        }),
        SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
        handle,
        json!({
            "version": 1,
            "effects": [],
            "events": [],
            "error": {
                "code": "requestNotPending",
                "message": "request navigation-1-1-1 is no longer pending",
            },
        }),
    );
    record_dispatch(
        &mut steps,
        "navigation-request-for-timeout",
        handle,
        json!({
            "version": 1,
            "observation": "navigationRequest",
            "url": "https://two.invalid/",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainNavigationCompletion",
                    "navigationId": "navigation-1-1-2",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "navigation-1-1-2",
                    "kind": "navigation",
                    "timeoutMs": 5000,
                },
            ],
            "events": [{
                "name": "navigationRequested",
                "payload": {
                    "navigationId": "navigation-1-1-2",
                    "url": "https://two.invalid/",
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "navigation-timeout-default-allow",
        handle,
        json!({
            "version": 1,
            "observation": "nativeTimeout",
            "requestId": "navigation-1-1-2",
            "kind": "navigation",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{
                "name": "resolveNavigationCompletion",
                "navigationId": "navigation-1-1-2",
                "allow": true,
            }],
            "events": [],
        }),
    );
    record_dispatch(
        &mut steps,
        "late-navigation-response",
        handle,
        json!({
            "version": 1,
            "command": "resolveNavigationRequest",
            "navigationId": "navigation-1-1-2",
            "allow": false,
        }),
        SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
        handle,
        json!({
            "version": 1,
            "effects": [],
            "events": [],
            "error": {
                "code": "requestNotPending",
                "message": "request navigation-1-1-2 is no longer pending",
            },
        }),
    );
    record_dispatch(
        &mut steps,
        "confirm-dialog-request",
        handle,
        json!({
            "version": 1,
            "observation": "simpleDialogRequest",
            "kind": "confirm",
            "message": "Continue?",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainDialogCompletion",
                    "dialogId": "dialog-1-1-3",
                    "kind": "confirm",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "dialog-1-1-3",
                    "kind": "dialog",
                    "timeoutMs": 60000,
                },
            ],
            "events": [{
                "name": "simpleDialogRequested",
                "payload": {
                    "dialogId": "dialog-1-1-3",
                    "kind": "confirm",
                    "message": "Continue?",
                    "defaultValue": null,
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "wrong-kind-navigation-response",
        handle,
        json!({
            "version": 1,
            "command": "resolveNavigationRequest",
            "navigationId": "dialog-1-1-3",
            "allow": true,
        }),
        SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
        handle,
        json!({
            "version": 1,
            "effects": [],
            "events": [],
            "error": {
                "code": "wrongRequestKind",
                "message": "request dialog-1-1-3 is dialog, not navigation",
            },
        }),
    );
    record_dispatch(
        &mut steps,
        "confirm-dialog-response",
        handle,
        json!({
            "version": 1,
            "command": "resolveSimpleDialog",
            "dialogId": "dialog-1-1-3",
            "confirmed": true,
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{
                "name": "resolveDialogCompletion",
                "dialogId": "dialog-1-1-3",
                "kind": "confirm",
                "confirmed": true,
                "promptValue": null,
            }],
            "events": [{
                "name": "simpleDialogDismissed",
                "payload": {"dialogId": "dialog-1-1-3"},
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "prompt-dialog-request",
        handle,
        json!({
            "version": 1,
            "observation": "simpleDialogRequest",
            "kind": "prompt",
            "message": "Name?",
            "defaultValue": "Servo",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainDialogCompletion",
                    "dialogId": "dialog-1-1-4",
                    "kind": "prompt",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "dialog-1-1-4",
                    "kind": "dialog",
                    "timeoutMs": 60000,
                },
            ],
            "events": [{
                "name": "simpleDialogRequested",
                "payload": {
                    "dialogId": "dialog-1-1-4",
                    "kind": "prompt",
                    "message": "Name?",
                    "defaultValue": "Servo",
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "prompt-dialog-default-cancel",
        handle,
        json!({
            "version": 1,
            "observation": "nativeTimeout",
            "requestId": "dialog-1-1-4",
            "kind": "dialog",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{
                "name": "resolveDialogCompletion",
                "dialogId": "dialog-1-1-4",
                "kind": "prompt",
                "confirmed": false,
                "promptValue": null,
            }],
            "events": [{
                "name": "simpleDialogDismissed",
                "payload": {"dialogId": "dialog-1-1-4"},
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "evaluation-request",
        handle,
        json!({
            "version": 1,
            "command": "evaluateJavaScript",
            "evaluationId": "eval-complete",
            "script": "1 + 1",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{
                "name": "evaluateJavaScript",
                "requestId": "evaluation-1-1-5",
                "evaluationId": "eval-complete",
                "script": "1 + 1",
            }],
            "events": [],
        }),
    );
    record_dispatch(
        &mut steps,
        "evaluation-completion",
        handle,
        json!({
            "version": 1,
            "observation": "javaScriptEvaluationResult",
            "requestId": "evaluation-1-1-5",
            "ok": true,
            "valueJson": "2",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [],
            "events": [{
                "name": "javascriptEvaluationResult",
                "payload": {
                    "evaluationId": "eval-complete",
                    "ok": true,
                    "valueJson": "2",
                    "errorType": null,
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "pending-navigation-before-reset",
        handle,
        json!({
            "version": 1,
            "observation": "navigationRequest",
            "url": "https://reset.invalid/",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainNavigationCompletion",
                    "navigationId": "navigation-1-1-6",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "navigation-1-1-6",
                    "kind": "navigation",
                    "timeoutMs": 5000,
                },
            ],
            "events": [{
                "name": "navigationRequested",
                "payload": {
                    "navigationId": "navigation-1-1-6",
                    "url": "https://reset.invalid/",
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "pending-alert-before-reset",
        handle,
        json!({
            "version": 1,
            "observation": "simpleDialogRequest",
            "kind": "alert",
            "message": "Notice",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainDialogCompletion",
                    "dialogId": "dialog-1-1-7",
                    "kind": "alert",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "dialog-1-1-7",
                    "kind": "dialog",
                    "timeoutMs": 60000,
                },
            ],
            "events": [{
                "name": "simpleDialogRequested",
                "payload": {
                    "dialogId": "dialog-1-1-7",
                    "kind": "alert",
                    "message": "Notice",
                    "defaultValue": null,
                },
            }],
        }),
    );
    record_dispatch(
        &mut steps,
        "pending-evaluation-before-reset",
        handle,
        json!({
            "version": 1,
            "command": "evaluateJavaScript",
            "evaluationId": "eval-reset",
            "script": "location.href",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [{
                "name": "evaluateJavaScript",
                "requestId": "evaluation-1-1-8",
                "evaluationId": "eval-reset",
                "script": "location.href",
            }],
            "events": [],
        }),
    );
    record_dispatch(
        &mut steps,
        "reset",
        handle,
        json!({"version": 1, "lifecycle": "reset"}),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "resolveNavigationCompletion",
                    "navigationId": "navigation-1-1-6",
                    "allow": true,
                },
                {
                    "name": "resolveDialogCompletion",
                    "dialogId": "dialog-1-1-7",
                    "kind": "alert",
                    "confirmed": true,
                    "promptValue": null,
                },
                {
                    "name": "invalidateEvaluation",
                    "requestId": "evaluation-1-1-8",
                },
                {"name": "reset", "generation": 2},
            ],
            "events": [
                {
                    "name": "simpleDialogDismissed",
                    "payload": {"dialogId": "dialog-1-1-7"},
                },
                {
                    "name": "javascriptEvaluationResult",
                    "payload": {
                        "evaluationId": "eval-reset",
                        "ok": false,
                        "valueJson": null,
                        "errorType": "WebViewNotReady",
                    },
                },
            ],
        }),
    );
    record_dispatch(
        &mut steps,
        "stale-generation-response",
        handle,
        json!({
            "version": 1,
            "command": "resolveNavigationRequest",
            "navigationId": "navigation-1-1-6",
            "allow": false,
        }),
        SERVOKIT_CONTROLLER_INVALID_ARGUMENT,
        handle,
        json!({
            "version": 1,
            "effects": [],
            "events": [],
            "error": {
                "code": "staleRequest",
                "message": "request navigation-1-1-6 belongs to an inactive generation",
            },
        }),
    );
    record_dispatch(
        &mut steps,
        "pending-navigation-before-destroy",
        handle,
        json!({
            "version": 1,
            "observation": "navigationRequest",
            "url": "https://destroy.invalid/",
        }),
        SERVOKIT_CONTROLLER_OK,
        handle,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "retainNavigationCompletion",
                    "navigationId": "navigation-1-2-9",
                },
                {
                    "name": "scheduleTimeout",
                    "requestId": "navigation-1-2-9",
                    "kind": "navigation",
                    "timeoutMs": 5000,
                },
            ],
            "events": [{
                "name": "navigationRequested",
                "payload": {
                    "navigationId": "navigation-1-2-9",
                    "url": "https://destroy.invalid/",
                },
            }],
        }),
    );
    record_destroy(
        &mut steps,
        handle,
        SERVOKIT_CONTROLLER_OK,
        0,
        json!({
            "version": 1,
            "effects": [
                {
                    "name": "resolveNavigationCompletion",
                    "navigationId": "navigation-1-2-9",
                    "allow": true,
                },
                {"name": "destroy"},
            ],
            "events": [],
        }),
    );
    record_dispatch(
        &mut steps,
        "stale-handle-after-destroy",
        handle,
        json!({"version": 1, "command": "reload"}),
        SERVOKIT_CONTROLLER_STALE_HANDLE,
        0,
        json!({
            "version": 1,
            "effects": [],
            "events": [],
            "error": {
                "code": "staleHandle",
                "message": "controller handle is not live",
            },
        }),
    );

    let transcript = json!({
        "baselineCommit": "ec673e94590e8861e273d73c36835d1cd6f53d86",
        "contractVersion": 1,
        "generatedBy": "crates/servokit-controller-ffi/tests/reducer_abi_transcript.rs",
        "generationCommand": "SERVOKIT_CONTROLLER_TRANSCRIPT_PATH=<output> cargo test --manifest-path crates/Cargo.toml -p servokit-controller-ffi --test reducer_abi_transcript -- --exact reducer_abi_transcript",
        "freshProcessFirstHandle": 1,
        "steps": steps,
    });

    if let Some(path) = std::env::var_os("SERVOKIT_CONTROLLER_TRANSCRIPT_PATH") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("transcript parent must be writable");
        }
        let bytes = serde_json::to_vec_pretty(&transcript).expect("transcript must serialize");
        fs::write(path, bytes).expect("transcript output must be writable");
    }
}
