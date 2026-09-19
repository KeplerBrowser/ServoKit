#define NOMINMAX
#include <windows.h>

#include "servokit_desktop_private.h"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <limits>

namespace {

void trace(const char *stage) {
  std::fprintf(stderr, "stage=%s\n", stage);
  std::fflush(stderr);
}

void wake_callback(void *, std::uint64_t) noexcept {}
void event_callback(void *, std::uint64_t, std::uint64_t,
                    const char *) noexcept {}

bool expect_status(const char *operation,
                   ServokitDesktopPrivateStatus actual,
                   ServokitDesktopPrivateStatus expected) {
  if (actual == expected) {
    return true;
  }
  std::fprintf(stderr, "%s: expected status %d, got %d\n", operation,
               static_cast<int>(expected), static_cast<int>(actual));
  return false;
}

LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM wparam,
                             LPARAM lparam) {
  return DefWindowProcW(window, message, wparam, lparam);
}

bool run_lifecycle(HWND child, HINSTANCE instance) {
  trace("create-start");
  ServokitDesktopPrivateHost *host = nullptr;
  std::uint64_t token = 0;
  const ServokitDesktopPrivateCallbacks callbacks{nullptr, wake_callback};
  auto destroy_if_live = [&]() {
    if (host != nullptr && token != 0) {
      const auto destroy_status = servokit_desktop_private_destroy(host, token);
      host = nullptr;
      token = 0;
      return destroy_status;
    }
    return SERVOKIT_DESKTOP_PRIVATE_OK;
  };
  auto fail = [&]() {
    destroy_if_live();
    return false;
  };

  auto status = servokit_desktop_private_create(callbacks, &host, &token);
  trace("create-complete");
  if (!expect_status("create", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }
  const std::uint64_t wrong_token =
      token == std::numeric_limits<std::uint64_t>::max() ? token - 1
                                                         : token + 1;

  const ServokitDesktopPrivateNativeSurface surface{
      child, reinterpret_cast<void *>(instance)};
  const ServokitDesktopPrivateViewport initial{40, 60, 640, 480, 2.0F};
  std::uint64_t first_generation = 0;
  status = servokit_desktop_private_attach(host, token, surface, initial,
                                           &first_generation);
  trace("attach-complete");
  if (!expect_status("attach", status, SERVOKIT_DESKTOP_PRIVATE_OK) ||
      first_generation == 0) {
    return fail();
  }

  status = servokit_desktop_private_update_viewport(host, wrong_token, initial);
  if (!expect_status("attached resize rejects stale token", status,
                     SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN)) {
    return fail();
  }

  std::uint64_t rejected_generation =
      std::numeric_limits<std::uint64_t>::max();
  status = servokit_desktop_private_attach(host, wrong_token, surface, initial,
                                           &rejected_generation);
  if (!expect_status("attached reattach rejects stale token", status,
                     SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN) ||
      rejected_generation != 0) {
    return fail();
  }

  const ServokitDesktopPrivateViewport resized{24, 36, 600, 400, 1.5F};
  status = servokit_desktop_private_update_viewport(host, token, resized);
  if (!expect_status("resize", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }

  ServokitDesktopPrivateInput wheel{};
  wheel.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_WHEEL;
  wheel.x = 20.0F;
  wheel.y = 120.0F;
  wheel.delta_x = 6.0;
  wheel.delta_y = 14.0;
  wheel.scroll_mode = SERVOKIT_DESKTOP_PRIVATE_SCROLL_PIXELS;
  status = servokit_desktop_private_dispatch_input(host, token, wheel);
  if (!expect_status("pixel wheel", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }

  status = servokit_desktop_private_pump(host, token);
  if (!expect_status("pump", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }
  status = servokit_desktop_private_drain_events(host, token, event_callback,
                                                 nullptr);
  if (!expect_status("drain", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }
  status = servokit_desktop_private_detach(host, token);
  trace("detach-complete");
  if (!expect_status("detach", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }

  std::uint64_t second_generation = 0;
  status = servokit_desktop_private_attach(host, token, surface, initial,
                                           &second_generation);
  trace("reattach-complete");
  if (!expect_status("reattach", status, SERVOKIT_DESKTOP_PRIVATE_OK) ||
      second_generation == 0 || second_generation == first_generation) {
    return fail();
  }
  status = servokit_desktop_private_detach(host, token);
  trace("second-detach-complete");
  if (!expect_status("second detach", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return fail();
  }
  status = servokit_desktop_private_destroy(host, token);
  trace("destroy-complete");
  if (!expect_status("destroy", status, SERVOKIT_DESKTOP_PRIVATE_OK)) {
    return false;
  }
  const auto destroyed_host = host;
  const auto destroyed_token = token;
  host = nullptr;
  token = 0;

  status = servokit_desktop_private_pump(destroyed_host, destroyed_token);
  trace("stale-pump-complete");
  return expect_status("destroyed host rejects stale token", status,
                       SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN);
}

} // namespace

int main() {
  trace("window-create-start");
  HINSTANCE instance = GetModuleHandleW(nullptr);
  const wchar_t class_name[] = L"ServoKitPrivateLifecycle";
  WNDCLASSW window_class{};
  window_class.hInstance = instance;
  window_class.lpfnWndProc = window_proc;
  window_class.lpszClassName = class_name;
  if (RegisterClassW(&window_class) == 0) {
    return EXIT_FAILURE;
  }

  HWND parent = CreateWindowExW(0, class_name, L"", WS_OVERLAPPEDWINDOW, 0, 0,
                                400, 320, nullptr, nullptr, instance, nullptr);
  HWND child = CreateWindowExW(0, class_name, L"", WS_CHILD | WS_VISIBLE, 20,
                               30, 320, 240, parent, nullptr, instance, nullptr);
  const bool passed = parent != nullptr && child != nullptr &&
                      run_lifecycle(child, instance);
  trace("lifecycle-complete");
  if (child != nullptr) {
    trace("child-window-destroy-start");
    DestroyWindow(child);
    trace("child-window-destroy-complete");
  }
  if (parent != nullptr) {
    trace("parent-window-destroy-start");
    DestroyWindow(parent);
    trace("parent-window-destroy-complete");
  }
  UnregisterClassW(class_name, instance);
  trace("window-class-unregister-complete");
  // The host and both HWNDs are fully destroyed at this point. Skip process-global
  // ANGLE DLL detach, which can block when ANGLE is loaded by this isolated
  // static-library test executable rather than by a long-lived application host.
  trace("process-exit");
  const UINT exit_code = passed ? EXIT_SUCCESS : EXIT_FAILURE;
  if (TerminateProcess(GetCurrentProcess(), exit_code) == FALSE) {
    return EXIT_FAILURE;
  }
  return EXIT_FAILURE;
}
