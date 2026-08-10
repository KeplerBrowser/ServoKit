#pragma once

#include <stddef.h>
#include <stdint.h>

#if defined(__APPLE__)
#include <TargetConditionals.h>
#endif

#if (defined(__APPLE__) && TARGET_OS_OSX) || defined(_WIN32)

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ServokitDesktopPrivateHost ServokitDesktopPrivateHost;

typedef enum ServokitDesktopPrivateStatus {
  SERVOKIT_DESKTOP_PRIVATE_OK = 0,
  SERVOKIT_DESKTOP_PRIVATE_NULL_POINTER = 1,
  SERVOKIT_DESKTOP_PRIVATE_INVALID_ARGUMENT = 2,
  SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN = 3,
  SERVOKIT_DESKTOP_PRIVATE_WRONG_THREAD = 4,
  SERVOKIT_DESKTOP_PRIVATE_BUSY = 5,
  SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR = 6,
  SERVOKIT_DESKTOP_PRIVATE_PANIC = 7,
} ServokitDesktopPrivateStatus;

/* Maximum size of the complete ControllerCommand v1 JSON envelope: 1 MiB. */
#define SERVOKIT_DESKTOP_PRIVATE_MAX_COMMAND_BYTES ((size_t)1048576)

#ifdef __cplusplus
typedef void (*ServokitDesktopPrivateWakeCallback)(void *context,
                                                    uint64_t token) noexcept;
typedef void (*ServokitDesktopPrivateEventCallback)(
    void *context, uint64_t token, uint64_t attachment_generation,
    const char *event_json) noexcept;
#else
typedef void (*ServokitDesktopPrivateWakeCallback)(void *context,
                                                    uint64_t token);
typedef void (*ServokitDesktopPrivateEventCallback)(
    void *context, uint64_t token, uint64_t attachment_generation,
    const char *event_json);
#endif

typedef struct ServokitDesktopPrivateCallbacks {
  void *context;
  ServokitDesktopPrivateWakeCallback wake;
} ServokitDesktopPrivateCallbacks;

typedef struct ServokitDesktopPrivateNativeSurface {
  void *window;
  void *display;
} ServokitDesktopPrivateNativeSurface;

typedef struct ServokitDesktopPrivateViewport {
  int32_t x;
  int32_t y;
  uint32_t width;
  uint32_t height;
  float scale_factor;
} ServokitDesktopPrivateViewport;

enum {
  SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_MOVE = 0,
  SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_BUTTON = 1,
  SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_WHEEL = 2,
  SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_LEAVE = 3,
  SERVOKIT_DESKTOP_PRIVATE_INPUT_KEYBOARD = 4,
  SERVOKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT = 5,
  SERVOKIT_DESKTOP_PRIVATE_INPUT_FOCUS = 6,
};

enum {
  SERVOKIT_DESKTOP_PRIVATE_RELEASED = 0,
  SERVOKIT_DESKTOP_PRIVATE_PRESSED = 1,
  SERVOKIT_DESKTOP_PRIVATE_SCROLL_LINES = 0,
  SERVOKIT_DESKTOP_PRIVATE_SCROLL_PIXELS = 1,
};

enum {
  SERVOKIT_DESKTOP_PRIVATE_BUTTON_PRIMARY = 0,
  SERVOKIT_DESKTOP_PRIVATE_BUTTON_SECONDARY = 1,
  SERVOKIT_DESKTOP_PRIVATE_BUTTON_MIDDLE = 2,
  SERVOKIT_DESKTOP_PRIVATE_BUTTON_BACK = 3,
  SERVOKIT_DESKTOP_PRIVATE_BUTTON_FORWARD = 4,
  SERVOKIT_DESKTOP_PRIVATE_BUTTON_OTHER = 5,
};

enum {
  SERVOKIT_DESKTOP_PRIVATE_KEY_CHARACTER = 0,
  SERVOKIT_DESKTOP_PRIVATE_KEY_NAMED = 1,
};

enum {
  SERVOKIT_DESKTOP_PRIVATE_KEY_BACKSPACE = 0,
  SERVOKIT_DESKTOP_PRIVATE_KEY_DELETE = 1,
  SERVOKIT_DESKTOP_PRIVATE_KEY_ENTER = 2,
  SERVOKIT_DESKTOP_PRIVATE_KEY_TAB = 3,
  SERVOKIT_DESKTOP_PRIVATE_KEY_ESCAPE = 4,
  SERVOKIT_DESKTOP_PRIVATE_KEY_SPACE = 5,
  SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_LEFT = 6,
  SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_RIGHT = 7,
  SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_UP = 8,
  SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_DOWN = 9,
};

typedef struct ServokitDesktopPrivateInput {
  uint32_t kind;
  float x;
  float y;
  double delta_x;
  double delta_y;
  uint32_t button_kind;
  uint32_t button_code;
  uint32_t action;
  uint32_t scroll_mode;
  uint32_t key_kind;
  uint32_t key_code;
  const char *text;
  uint8_t repeat;
  uint8_t is_composing;
  uint8_t is_focused;
} ServokitDesktopPrivateInput;

/*
 * This is package-private adapter glue, not a public C SDK. Create and call a
 * host only on its UI thread: the AppKit main thread or the thread that owns
 * the Win32 child HWND. UI-thread calls and event callbacks are serialized and
 * non-reentrant. Wake callbacks are serialized with other wake callbacks, but
 * may run off the UI thread and overlap UI-thread calls/event callbacks; their
 * context must be thread-safe and must only schedule UI-thread pump/drain work.
 * Callback context must remain valid until accepted destroy or a host-consuming
 * operation returns. Any panic after an operation accepts a live host/token is
 * host-consuming: callbacks are deactivated, the token becomes stale, and the
 * operation returns PANIC. Preflight rejection is not host-consuming.
 * Destroy and every host-consuming failure wait for in-flight wake callbacks
 * before deactivating the callback context. Every callback must
 * return normally: never throw a C++/Objective-C exception, unwind, or longjmp
 * across this C boundary. C++ and Objective-C++ adapters should use noexcept
 * callbacks and catch every exception inside the callback body. Objective-C++
 * adapters must contain both C++ exceptions and Objective-C exceptions with
 * the matching language catch mechanism.
 *
 * A successful attach borrows the native handles until an accepted detach or
 * destroy call returns. Attach failure never retains the newly supplied
 * handles. Accepted attach RUNTIME_ERROR or PANIC consumes the host and makes
 * its token stale; the app may release the supplied handles as soon as attach
 * returns and must create a new host rather than retry or destroy the old one.
 * Rejected NULL_POINTER, INVALID_ARGUMENT, STALE_TOKEN, WRONG_THREAD, or BUSY
 * preserves any prior live host state and does not retain the newly supplied
 * handles. An accepted call is made with a live host/token on its owner thread
 * while the host is not busy.
 * Accepted detach returning OK keeps a detached controller live. Accepted
 * detach returning RUNTIME_ERROR or PANIC consumes the host, so its token is
 * stale. In all three cases the app may release the handles immediately after
 * detach returns. A rejected NULL_POINTER, STALE_TOKEN, WRONG_THREAD, or BUSY
 * call leaves any live attachment unchanged, so it is not a release point.
 * Destroy consumes an accepted host even when runtime detach fails; native
 * handles passed to a successful attach must remain valid until destroy
 * returns, then may be released.
 * An update_viewport RUNTIME_ERROR from a detached/state precondition is
 * non-host-consuming and leaves the token valid. Once viewport preflight is
 * accepted, a runtime host failure is host-consuming: callbacks are deactivated
 * and every later export returns STALE_TOKEN.
 *
 * Viewport x/y/width/height and pointer x/y are child-NSView/HWND-local
 * physical pixels with a top-left origin; +x is right and +y is down. They are
 * never parent, window, screen, AppKit-point, or logical-DPI coordinates.
 * Native adapters own AppKit local-point conversion, Y-axis flipping, and
 * backing-scale conversion, and own any Win32 screen-to-client conversion.
 * With SCROLL_PIXELS, delta_x/delta_y are signed physical-pixel distances in
 * the same axes (+x right, +y down), not raw wheel ticks, lines, or AppKit
 * points. Native adapters convert/invert platform wheel values before calling
 * this bridge; the bridge forwards pixel deltas without scaling or inversion.
 *
 * For pointer buttons, button_kind selects a named button and button_code must
 * be zero, or selects BUTTON_OTHER and button_code must fit uint16_t. For
 * keyboard input, key_kind selects either non-empty UTF-8 text with key_code
 * zero, or a named key_code with text set to NULL. No platform key mapping is
 * performed by this bridge.
 *
 * Controller commands and responses cross this boundary only as one opaque
 * ControllerCommand v1 JSON envelope. command is a borrowed byte slice valid
 * for the synchronous call. Its length must be
 * 1..SERVOKIT_DESKTOP_PRIVATE_MAX_COMMAND_BYTES; the bytes must be UTF-8 but
 * need not be NUL-terminated. The desktop bridge rejects invalid
 * pointer/length/UTF-8 transport before calling the shared Rust parser, which
 * rejects invalid JSON and owns envelope version, command names,
 * command-specific payload validation, and dispatch. Platform adapters must
 * not parse that schema.
 *
 * Tokens and nonzero attachment generations are never reused. Attachment
 * generation output is required: attach writes zero on failure and a unique
 * nonzero value on success. Identifier exhaustion is permanent for the
 * process; later create/attach calls that need an identifier return
 * RUNTIME_ERROR. Runtime-synthesized surfaceAttached, surfaceResized, and
 * surfaceDetached events carry the attachment generation; all other
 * controller-owned events carry generation zero. Events from an older
 * attachment are not delivered after a successful reattach. Event callback
 * event_json is borrowed UTF-8. It is valid only synchronously during that
 * callback invocation and must be copied to retain it.
 */
ServokitDesktopPrivateStatus servokit_desktop_private_create(
    ServokitDesktopPrivateCallbacks callbacks,
    ServokitDesktopPrivateHost **out_host,
    uint64_t *out_token);
ServokitDesktopPrivateStatus servokit_desktop_private_destroy(
    ServokitDesktopPrivateHost *host, uint64_t token);
ServokitDesktopPrivateStatus servokit_desktop_private_attach(
    ServokitDesktopPrivateHost *host, uint64_t token,
    ServokitDesktopPrivateNativeSurface surface,
    ServokitDesktopPrivateViewport viewport,
    uint64_t *out_attachment_generation);
ServokitDesktopPrivateStatus servokit_desktop_private_update_viewport(
    ServokitDesktopPrivateHost *host, uint64_t token,
    ServokitDesktopPrivateViewport viewport);
ServokitDesktopPrivateStatus servokit_desktop_private_detach(
    ServokitDesktopPrivateHost *host, uint64_t token);
ServokitDesktopPrivateStatus
servokit_desktop_private_dispatch_controller_command(
    ServokitDesktopPrivateHost *host, uint64_t token,
    const uint8_t *command, size_t command_length);
ServokitDesktopPrivateStatus servokit_desktop_private_dispatch_input(
    ServokitDesktopPrivateHost *host, uint64_t token,
    ServokitDesktopPrivateInput input);
ServokitDesktopPrivateStatus servokit_desktop_private_pump(
    ServokitDesktopPrivateHost *host, uint64_t token);
ServokitDesktopPrivateStatus servokit_desktop_private_drain_events(
    ServokitDesktopPrivateHost *host, uint64_t token,
    ServokitDesktopPrivateEventCallback callback, void *context);

#ifdef __cplusplus
}
#endif

#endif
