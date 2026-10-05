#import <AppKit/AppKit.h>

#include "explorerkit_desktop_private.h"

#include <cmath>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <thread>

@interface ExplorerKitLifecycleView : NSView
@end

@implementation ExplorerKitLifecycleView
- (BOOL)isFlipped
{
  return YES;
}
@end

namespace {

void wake_callback(void *, std::uint64_t) noexcept {}

struct NavigationState {
  const char *url;
  bool completed;
};

void event_callback(void *context, std::uint64_t, std::uint64_t,
                    const char *event_json) noexcept {
  auto *state = static_cast<NavigationState *>(context);
  if (state != nullptr && event_json != nullptr &&
      std::strstr(event_json, "\"name\":\"urlChanged\"") != nullptr &&
      std::strstr(event_json, state->url) != nullptr) {
    state->completed = true;
  }
}

bool expect_status(const char *operation,
                   ExplorerKitDesktopPrivateStatus actual,
                   ExplorerKitDesktopPrivateStatus expected) {
  if (actual == expected) {
    return true;
  }
  std::fprintf(stderr, "%s: expected status %d, got %d\n", operation,
               static_cast<int>(expected), static_cast<int>(actual));
  return false;
}

bool dispatch_input(ExplorerKitDesktopPrivateHost *host, std::uint64_t token,
                    const char *operation,
                    ExplorerKitDesktopPrivateInput input) {
  return expect_status(
      operation,
      explorerkit_desktop_private_dispatch_input(host, token, input),
      EXPLORERKIT_DESKTOP_PRIVATE_OK);
}

bool await_navigation(ExplorerKitDesktopPrivateHost *host, std::uint64_t token,
                      const char *url) {
  NavigationState state{url, false};
  const auto deadline =
      std::chrono::steady_clock::now() + std::chrono::seconds(10);
  while (!state.completed && std::chrono::steady_clock::now() < deadline) {
    if (!expect_status("pump", explorerkit_desktop_private_pump(host, token),
                       EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
        !expect_status("drain",
                       explorerkit_desktop_private_drain_events(
                           host, token, event_callback, &state),
                       EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
      return false;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
  return state.completed;
}

NSPoint child_local_physical_point(NSView *child, NSPoint parent_point,
                                   CGFloat scale_factor) {
  const NSPoint local = [child convertPoint:parent_point
                                   fromView:[child superview]];
  return NSMakePoint(local.x * scale_factor, local.y * scale_factor);
}

bool run_lifecycle(NSView *child) {
  const NSPoint pointer =
      child_local_physical_point(child, NSMakePoint(30.0, 210.0), 2.0);
  if (std::abs(pointer.x - 20.0) > 0.01 ||
      std::abs(pointer.y - 120.0) > 0.01) {
    std::fprintf(stderr, "AppKit top-left physical conversion failed\n");
    return false;
  }

  ExplorerKitDesktopPrivateHost *host = nullptr;
  std::uint64_t token = 0;
  const ExplorerKitDesktopPrivateCallbacks callbacks{nullptr, wake_callback};
  auto status = explorerkit_desktop_private_create(callbacks, &host, &token);
  if (!expect_status("create", status, EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
    return false;
  }

  constexpr char command[] =
      R"({"version":1,"command":"loadUrl","url":"data:text/html,%3Ctitle%3EFirst%3C/title%3Efirst-lifecycle"})";
  status = explorerkit_desktop_private_dispatch_controller_command(
      host, token, reinterpret_cast<const std::uint8_t *>(command),
      sizeof(command) - 1);
  if (!expect_status("first detached navigation", status,
                     EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
    return false;
  }

  const ExplorerKitDesktopPrivateNativeSurface surface{
      (__bridge void *)child, nullptr};
  const ExplorerKitDesktopPrivateViewport initial{40, 60, 640, 480, 2.0F};
  std::uint64_t first_generation = 0;
  status = explorerkit_desktop_private_attach(host, token, surface, initial,
                                           &first_generation);
  if (!expect_status("attach", status, EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
      first_generation == 0) {
    return false;
  }
  if (!await_navigation(host, token, "first-lifecycle")) {
    std::fprintf(stderr, "first navigation did not complete\n");
    return false;
  }

  const ExplorerKitDesktopPrivateViewport resized{24, 36, 600, 400, 2.0F};
  status = explorerkit_desktop_private_update_viewport(host, token, resized);
  if (!expect_status("resize", status, EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
    return false;
  }

  ExplorerKitDesktopPrivateInput moved{};
  moved.kind = EXPLORERKIT_DESKTOP_PRIVATE_INPUT_POINTER_MOVE;
  moved.x = static_cast<float>(pointer.x);
  moved.y = static_cast<float>(pointer.y);
  if (!dispatch_input(host, token, "pointer", moved)) {
    return false;
  }

  ExplorerKitDesktopPrivateInput button = moved;
  button.kind = EXPLORERKIT_DESKTOP_PRIVATE_INPUT_POINTER_BUTTON;
  button.button_kind = EXPLORERKIT_DESKTOP_PRIVATE_BUTTON_PRIMARY;
  button.action = EXPLORERKIT_DESKTOP_PRIVATE_PRESSED;
  if (!dispatch_input(host, token, "button down", button)) {
    return false;
  }
  button.action = EXPLORERKIT_DESKTOP_PRIVATE_RELEASED;
  if (!dispatch_input(host, token, "button up", button)) {
    return false;
  }

  ExplorerKitDesktopPrivateInput wheel = moved;
  wheel.kind = EXPLORERKIT_DESKTOP_PRIVATE_INPUT_POINTER_WHEEL;
  wheel.delta_x = 6.0;
  wheel.delta_y = 14.0;
  wheel.scroll_mode = EXPLORERKIT_DESKTOP_PRIVATE_SCROLL_PIXELS;
  if (!dispatch_input(host, token, "pixel wheel", wheel)) {
    return false;
  }

  ExplorerKitDesktopPrivateInput focus{};
  focus.kind = EXPLORERKIT_DESKTOP_PRIVATE_INPUT_FOCUS;
  focus.is_focused = 1;
  if (!dispatch_input(host, token, "focus", focus)) {
    return false;
  }

  char text[] = "a";
  ExplorerKitDesktopPrivateInput key{};
  key.kind = EXPLORERKIT_DESKTOP_PRIVATE_INPUT_KEYBOARD;
  key.action = EXPLORERKIT_DESKTOP_PRIVATE_PRESSED;
  key.key_kind = EXPLORERKIT_DESKTOP_PRIVATE_KEY_CHARACTER;
  key.text = text;
  if (!dispatch_input(host, token, "key", key)) {
    return false;
  }

  ExplorerKitDesktopPrivateInput ime{};
  ime.kind = EXPLORERKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT;
  ime.text = text;
  if (!dispatch_input(host, token, "IME commit", ime)) {
    return false;
  }

  status = explorerkit_desktop_private_detach(host, token);
  if (!expect_status("detach", status, EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
    return false;
  }

  std::uint64_t previous_generation = first_generation;
  for (int cycle = 0; cycle < 2; ++cycle) {
    std::uint64_t generation = 0;
    status = explorerkit_desktop_private_attach(host, token, surface, initial,
                                             &generation);
    if (!expect_status("reattach", status, EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
        generation == 0 || generation == previous_generation) {
      return false;
    }
    previous_generation = generation;
    status = explorerkit_desktop_private_detach(host, token);
    if (!expect_status("recycle detach", status,
                       EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
      return false;
    }
  }
  status = explorerkit_desktop_private_destroy(host, token);
  if (!expect_status("destroy", status, EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
      !expect_status("stale old token",
                     explorerkit_desktop_private_pump(host, token),
                     EXPLORERKIT_DESKTOP_PRIVATE_STALE_TOKEN)) {
    return false;
  }

  ExplorerKitDesktopPrivateHost *replacement_host = nullptr;
  std::uint64_t replacement_token = 0;
  status = explorerkit_desktop_private_create(callbacks, &replacement_host,
                                           &replacement_token);
  if (!expect_status("replacement create", status,
                     EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
      replacement_token == token) {
    return false;
  }

  constexpr char replacement_command[] =
      R"({"version":1,"command":"loadUrl","url":"data:text/html,%3Ctitle%3ESecond%3C/title%3Esecond-lifecycle"})";
  status = explorerkit_desktop_private_dispatch_controller_command(
      replacement_host, replacement_token,
      reinterpret_cast<const std::uint8_t *>(replacement_command),
      sizeof(replacement_command) - 1);
  std::uint64_t replacement_generation = 0;
  if (!expect_status("second detached navigation", status,
                     EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
      !expect_status("replacement attach",
                     explorerkit_desktop_private_attach(
                         replacement_host, replacement_token, surface, initial,
                         &replacement_generation),
                     EXPLORERKIT_DESKTOP_PRIVATE_OK) ||
      replacement_generation == 0 ||
      !await_navigation(replacement_host, replacement_token,
                        "second-lifecycle") ||
      !expect_status("replacement detach",
                     explorerkit_desktop_private_detach(replacement_host,
                                                    replacement_token),
                     EXPLORERKIT_DESKTOP_PRIVATE_OK)) {
    return false;
  }

  return expect_status(
      "replacement destroy",
      explorerkit_desktop_private_destroy(replacement_host, replacement_token),
      EXPLORERKIT_DESKTOP_PRIVATE_OK);
}

} // namespace

int main(int argc, char **argv) {
  @autoreleasepool {
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
    [NSApp finishLaunching];

    NSWindow *window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0.0, 0.0, 400.0, 320.0)
                  styleMask:(NSWindowStyleMaskTitled |
                             NSWindowStyleMaskClosable)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    NSView *parent =
        [[NSView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 400.0, 320.0)];
    NSView *child = [[ExplorerKitLifecycleView alloc]
        initWithFrame:NSMakeRect(20.0, 30.0, 320.0, 240.0)];
    [child setWantsLayer:YES];
    [parent addSubview:child];
    [window setContentView:parent];
    [window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
    [window displayIfNeeded];

    const bool passed = run_lifecycle(child);
    [child removeFromSuperview];
    [window close];
    if (argc == 2) {
      if (FILE *result = std::fopen(argv[1], "w")) {
        std::fputs(passed ? "passed\n" : "failed\n", result);
        std::fclose(result);
      }
    }
    return passed ? EXIT_SUCCESS : EXIT_FAILURE;
  }
}
