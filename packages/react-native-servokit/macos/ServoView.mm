#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <string>
#include <vector>

#import <AppKit/AppKit.h>
#import <React/RCTViewComponentView.h>
#import <react/renderer/components/ServoViewSpec/ComponentDescriptors.h>
#import <react/renderer/components/ServoViewSpec/EventEmitters.h>
#import <react/renderer/components/ServoViewSpec/Props.h>
#import <react/renderer/components/ServoViewSpec/RCTComponentViewHelpers.h>

#import <ServoKit/servokit_desktop_private.h>

#import "ServoHostDestroy.h"

using namespace facebook::react;

@class ServoView;

@interface ServoKitWakeContext : NSObject
@property (atomic, weak) ServoView *view;
@end

@interface ServoKitContentView : NSView <NSTextInputClient>
@property (nonatomic, weak) ServoView *servoView;
- (void)resetTextInput;
@end

@interface ServoView : RCTViewComponentView <RCTServoViewViewProtocol>
- (void)installAdapterLifetimes;
- (void)syncSurface;
- (void)loadURLIfNeeded;
- (void)loadURLIfNeededAllowingDetached:(BOOL)allowDetached;
- (ServokitDesktopPrivateStatus)dispatchControllerCommand:(NSString *)commandJson;
- (void)pumpForToken:(uint64_t)token;
- (void)dispatchInput:(ServokitDesktopPrivateInput)input;
- (void)receiveEventJson:(const char *)eventJson
                   token:(uint64_t)token
      attachmentGeneration:(uint64_t)attachmentGeneration;
@end

@implementation ServoKitWakeContext
@end

static std::string ServoKitStdString(id value)
{
  if (![value isKindOfClass:NSString.class]) {
    return {};
  }
  return std::string([(NSString *)value UTF8String] ?: "");
}

static NSDictionary<NSString *, id> *ServoKitDictionary(id value)
{
  return [value isKindOfClass:NSDictionary.class] ? value : nil;
}

static NSArray *ServoKitArray(id value)
{
  return [value isKindOfClass:NSArray.class] ? value : nil;
}

static NSString *ServoKitString(id value)
{
  return [value isKindOfClass:NSString.class] ? value : nil;
}

static BOOL ServoKitBool(id value)
{
  return [value isKindOfClass:NSNumber.class] && [(NSNumber *)value boolValue];
}

static int ServoKitInt(id value)
{
  if (![value isKindOfClass:NSNumber.class]) {
    return 0;
  }
  NSInteger integer = [(NSNumber *)value integerValue];
  return static_cast<int>(std::clamp<NSInteger>(
      integer, std::numeric_limits<int>::min(), std::numeric_limits<int>::max()));
}

static void ServoKitWake(void *context, uint64_t token) noexcept
{
  try {
    @try {
      ServoKitWakeContext *wakeContext = (__bridge ServoKitWakeContext *)context;
      dispatch_async(dispatch_get_main_queue(), ^{
        [wakeContext.view pumpForToken:token];
      });
    } @catch (__unused NSException *exception) {
    }
  } catch (...) {
  }
}

static void ServoKitEvent(void *context,
                          uint64_t token,
                          uint64_t attachmentGeneration,
                          const char *eventJson) noexcept
{
  try {
    @try {
      ServoView *view = (__bridge ServoView *)context;
      [view receiveEventJson:eventJson
                       token:token
        attachmentGeneration:attachmentGeneration];
    } @catch (__unused NSException *exception) {
    }
  } catch (...) {
  }
}

static BOOL ServoKitNamedKey(NSEvent *event, uint32_t *keyCode)
{
  NSString *characters = event.charactersIgnoringModifiers;
  if (characters.length == 0) {
    return NO;
  }

  switch ([characters characterAtIndex:0]) {
    case NSBackspaceCharacter:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_BACKSPACE;
      return YES;
    case NSDeleteCharacter:
    case NSDeleteFunctionKey:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_DELETE;
      return YES;
    case NSEnterCharacter:
    case NSCarriageReturnCharacter:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ENTER;
      return YES;
    case NSTabCharacter:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_TAB;
      return YES;
    case 0x1b:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ESCAPE;
      return YES;
    case ' ':
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_SPACE;
      return YES;
    case NSLeftArrowFunctionKey:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_LEFT;
      return YES;
    case NSRightArrowFunctionKey:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_RIGHT;
      return YES;
    case NSUpArrowFunctionKey:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_UP;
      return YES;
    case NSDownArrowFunctionKey:
      *keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_DOWN;
      return YES;
    default:
      return NO;
  }
}

@implementation ServoKitContentView {
  NSMutableAttributedString *_markedText;
  NSString *_interpretedText;
  NSRange _selectedRange;
  NSTrackingArea *_trackingArea;
  BOOL _compositionActive;
  BOOL _interpretedTextWasComposing;
  BOOL _interpretingKeyEvent;
}

- (instancetype)initWithFrame:(NSRect)frame
{
  if ((self = [super initWithFrame:frame])) {
    _markedText = [[NSMutableAttributedString alloc] init];
    _selectedRange = NSMakeRange(0, 0);
    self.wantsLayer = YES;
  }
  return self;
}

- (BOOL)isFlipped
{
  return YES;
}

- (BOOL)acceptsFirstResponder
{
  return YES;
}

- (BOOL)acceptsFirstMouse:(NSEvent *)event
{
  return YES;
}

- (void)viewDidMoveToWindow
{
  [super viewDidMoveToWindow];
  [self updateTrackingAreas];
  [self.servoView syncSurface];
}

- (void)viewDidChangeBackingProperties
{
  [super viewDidChangeBackingProperties];
  [self.servoView syncSurface];
}

- (void)updateTrackingAreas
{
  [super updateTrackingAreas];
  if (_trackingArea != nil) {
    [self removeTrackingArea:_trackingArea];
  }
  NSTrackingAreaOptions options =
      NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved | NSTrackingActiveInKeyWindow |
      NSTrackingInVisibleRect;
  _trackingArea = [[NSTrackingArea alloc] initWithRect:NSZeroRect
                                               options:options
                                                 owner:self
                                              userInfo:nil];
  [self addTrackingArea:_trackingArea];
}

- (CGFloat)backingScale
{
  return self.window.backingScaleFactor ?: NSScreen.mainScreen.backingScaleFactor ?: 1.0;
}

- (NSPoint)physicalPointForEvent:(NSEvent *)event
{
  NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
  CGFloat scale = self.backingScale;
  return NSMakePoint(point.x * scale, point.y * scale);
}

- (void)dispatchPointerEvent:(NSEvent *)event kind:(uint32_t)kind action:(uint32_t)action
{
  NSPoint point = [self physicalPointForEvent:event];
  ServokitDesktopPrivateInput input = {};
  input.kind = kind;
  input.x = point.x;
  input.y = point.y;
  input.action = action;

  NSInteger button = event.buttonNumber;
  if (button == 0) {
    input.button_kind = SERVOKIT_DESKTOP_PRIVATE_BUTTON_PRIMARY;
  } else if (button == 1) {
    input.button_kind = SERVOKIT_DESKTOP_PRIVATE_BUTTON_SECONDARY;
  } else if (button == 2) {
    input.button_kind = SERVOKIT_DESKTOP_PRIVATE_BUTTON_MIDDLE;
  } else if (button == 3) {
    input.button_kind = SERVOKIT_DESKTOP_PRIVATE_BUTTON_BACK;
  } else if (button == 4) {
    input.button_kind = SERVOKIT_DESKTOP_PRIVATE_BUTTON_FORWARD;
  } else {
    input.button_kind = SERVOKIT_DESKTOP_PRIVATE_BUTTON_OTHER;
    input.button_code = static_cast<uint32_t>(
        std::clamp<NSInteger>(button, 0, std::numeric_limits<uint16_t>::max()));
  }

  [self.servoView dispatchInput:input];
}

- (void)mouseMoved:(NSEvent *)event
{
  [self dispatchPointerEvent:event
                        kind:SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_MOVE
                      action:SERVOKIT_DESKTOP_PRIVATE_RELEASED];
}

- (void)mouseDragged:(NSEvent *)event
{
  [self mouseMoved:event];
}

- (void)rightMouseDragged:(NSEvent *)event
{
  [self mouseMoved:event];
}

- (void)otherMouseDragged:(NSEvent *)event
{
  [self mouseMoved:event];
}

- (void)mouseExited:(NSEvent *)event
{
  ServokitDesktopPrivateInput input = {};
  input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_LEAVE;
  [self.servoView dispatchInput:input];
}

- (void)mouseDown:(NSEvent *)event
{
  [self.window makeFirstResponder:self];
  [self dispatchPointerEvent:event
                        kind:SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_BUTTON
                      action:SERVOKIT_DESKTOP_PRIVATE_PRESSED];
}

- (void)mouseUp:(NSEvent *)event
{
  [self dispatchPointerEvent:event
                        kind:SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_BUTTON
                      action:SERVOKIT_DESKTOP_PRIVATE_RELEASED];
}

- (void)rightMouseDown:(NSEvent *)event
{
  [self mouseDown:event];
}

- (void)rightMouseUp:(NSEvent *)event
{
  [self mouseUp:event];
}

- (void)otherMouseDown:(NSEvent *)event
{
  [self mouseDown:event];
}

- (void)otherMouseUp:(NSEvent *)event
{
  [self mouseUp:event];
}

- (void)scrollWheel:(NSEvent *)event
{
  NSPoint point = [self physicalPointForEvent:event];
  CGFloat scale = self.backingScale;
  ServokitDesktopPrivateInput input = {};
  input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_WHEEL;
  input.x = point.x;
  input.y = point.y;
  input.scroll_mode = event.hasPreciseScrollingDeltas
      ? SERVOKIT_DESKTOP_PRIVATE_SCROLL_PIXELS
      : SERVOKIT_DESKTOP_PRIVATE_SCROLL_LINES;
  input.delta_x = event.scrollingDeltaX * (event.hasPreciseScrollingDeltas ? scale : 1.0);
  input.delta_y = event.scrollingDeltaY * (event.hasPreciseScrollingDeltas ? scale : 1.0);
  [self.servoView dispatchInput:input];
}

- (void)dispatchKeyboardEvent:(NSEvent *)event
                       action:(uint32_t)action
                characterText:(NSString *)characterText
{
  ServokitDesktopPrivateInput input = {};
  input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_KEYBOARD;
  input.action = action;
  input.repeat = event.isARepeat;
  input.is_composing = self.hasMarkedText;

  uint32_t keyCode = 0;
  if (ServoKitNamedKey(event, &keyCode)) {
    input.key_kind = SERVOKIT_DESKTOP_PRIVATE_KEY_NAMED;
    input.key_code = keyCode;
    [self.servoView dispatchInput:input];
    return;
  }

  NSString *characters = characterText ?: event.charactersIgnoringModifiers;
  if (characters.length == 0) {
    return;
  }
  input.key_kind = SERVOKIT_DESKTOP_PRIVATE_KEY_CHARACTER;
  input.text = characters.UTF8String;
  [self.servoView dispatchInput:input];
}

- (void)keyDown:(NSEvent *)event
{
  uint32_t keyCode = 0;
  if (ServoKitNamedKey(event, &keyCode)) {
    [self dispatchKeyboardEvent:event
                         action:SERVOKIT_DESKTOP_PRIVATE_PRESSED
                  characterText:nil];
    [self interpretKeyEvents:@[ event ]];
    return;
  }

  _interpretedText = nil;
  _interpretedTextWasComposing = NO;
  _interpretingKeyEvent = YES;
  [self interpretKeyEvents:@[ event ]];
  _interpretingKeyEvent = NO;

  if (_interpretedText.length > 0) {
    if (_interpretedTextWasComposing) {
      ServokitDesktopPrivateInput input = {};
      input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT;
      input.text = _interpretedText.UTF8String;
      [self.servoView dispatchInput:input];
    } else {
      [self dispatchKeyboardEvent:event
                           action:SERVOKIT_DESKTOP_PRIVATE_PRESSED
                    characterText:_interpretedText];
    }
  } else if (!_compositionActive) {
    [self dispatchKeyboardEvent:event
                         action:SERVOKIT_DESKTOP_PRIVATE_PRESSED
                  characterText:nil];
  }
  _interpretedText = nil;
}

- (void)keyUp:(NSEvent *)event
{
  [self dispatchKeyboardEvent:event
                       action:SERVOKIT_DESKTOP_PRIVATE_RELEASED
                characterText:nil];
}

- (BOOL)becomeFirstResponder
{
  BOOL accepted = [super becomeFirstResponder];
  if (accepted) {
    ServokitDesktopPrivateInput input = {};
    input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_FOCUS;
    input.is_focused = 1;
    [self.servoView dispatchInput:input];
  }
  return accepted;
}

- (BOOL)resignFirstResponder
{
  BOOL accepted = [super resignFirstResponder];
  if (accepted) {
    ServokitDesktopPrivateInput input = {};
    input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_FOCUS;
    input.is_focused = 0;
    [self.servoView dispatchInput:input];
  }
  return accepted;
}

- (void)insertText:(id)string replacementRange:(NSRange)replacementRange
{
  NSString *text = [string isKindOfClass:NSAttributedString.class]
      ? [(NSAttributedString *)string string]
      : ServoKitString(string);
  if (text.length > 0) {
    BOOL wasComposing = _compositionActive || self.hasMarkedText;
    if (_interpretingKeyEvent) {
      _interpretedText = [text copy];
      _interpretedTextWasComposing = wasComposing;
    } else {
      ServokitDesktopPrivateInput input = {};
      input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT;
      input.text = text.UTF8String;
      [self.servoView dispatchInput:input];
    }
  }
  [self resetTextInput];
}

- (void)setMarkedText:(id)string
        selectedRange:(NSRange)selectedRange
      replacementRange:(NSRange)replacementRange
{
  NSAttributedString *value = [string isKindOfClass:NSAttributedString.class]
      ? string
      : [[NSAttributedString alloc] initWithString:ServoKitString(string) ?: @""];
  [_markedText setAttributedString:value];
  _selectedRange = selectedRange;
  _compositionActive = YES;
}

- (void)unmarkText
{
  [self resetTextInput];
}

- (void)resetTextInput
{
  [_markedText setAttributedString:[[NSAttributedString alloc] initWithString:@""]];
  _selectedRange = NSMakeRange(0, 0);
  _compositionActive = NO;
}

- (NSRange)selectedRange
{
  return _selectedRange;
}

- (NSRange)markedRange
{
  return _markedText.length > 0 ? NSMakeRange(0, _markedText.length) : NSMakeRange(NSNotFound, 0);
}

- (BOOL)hasMarkedText
{
  return _markedText.length > 0;
}

- (NSArray<NSAttributedStringKey> *)validAttributesForMarkedText
{
  return @[];
}

- (NSAttributedString *)attributedSubstringForProposedRange:(NSRange)range
                                               actualRange:(NSRangePointer)actualRange
{
  if (actualRange != nullptr) {
    *actualRange = NSMakeRange(NSNotFound, 0);
  }
  return nil;
}

- (NSUInteger)characterIndexForPoint:(NSPoint)point
{
  return 0;
}

- (NSRect)firstRectForCharacterRange:(NSRange)range actualRange:(NSRangePointer)actualRange
{
  if (actualRange != nullptr) {
    *actualRange = range;
  }
  NSRect rect = [self convertRect:self.bounds toView:nil];
  return self.window != nil ? [self.window convertRectToScreen:rect] : rect;
}

- (void)doCommandBySelector:(SEL)selector
{
}

@end

@implementation ServoView {
  ServoKitContentView *_servoContentView;
  ServoKitWakeContext *_wakeContext;
  NSString *_url;
  NSString *_loadedURL;
  ServokitDesktopPrivateHost *_host;
  uint64_t _token;
  uint64_t _attachmentGeneration;
  ServokitDesktopPrivateViewport _viewport;
  BOOL _isPumping;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<ServoViewComponentDescriptor>();
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if ((self = [super initWithFrame:frame])) {
    _props = ServoViewShadowNode::defaultSharedProps();
    [self installAdapterLifetimes];
    [self createHostIfNeeded];
  }
  return self;
}

- (void)installAdapterLifetimes
{
  _wakeContext = [[ServoKitWakeContext alloc] init];
  _wakeContext.view = self;
  _servoContentView = [[ServoKitContentView alloc] initWithFrame:self.bounds];
  _servoContentView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  _servoContentView.servoView = self;
  self.contentView = _servoContentView;
}

- (void)dealloc
{
  _wakeContext.view = nil;
  _servoContentView.servoView = nil;
  [self destroyHost];
}

- (BOOL)createHostIfNeeded
{
  if (_host != nullptr) {
    return YES;
  }
  if (![NSThread isMainThread]) {
    return NO;
  }

  ServokitDesktopPrivateCallbacks callbacks = {
      .context = (__bridge void *)_wakeContext,
      .wake = ServoKitWake,
  };
  ServokitDesktopPrivateHost *host = nullptr;
  uint64_t token = 0;
  ServokitDesktopPrivateStatus status =
      servokit_desktop_private_create(callbacks, &host, &token);
  if (status != SERVOKIT_DESKTOP_PRIVATE_OK) {
    return NO;
  }
  _host = host;
  _token = token;
  return YES;
}

- (void)destroyHost
{
  if (ServoKitDestroyHost(
          &_host,
          &_token,
          _wakeContext,
          _servoContentView,
          servokit_desktop_private_destroy)) {
    [self hostWasConsumed];
  }
}

- (void)hostWasConsumed
{
  _host = nullptr;
  _token = 0;
  _attachmentGeneration = 0;
  _loadedURL = nil;
}

- (ServokitDesktopPrivateViewport)currentViewport
{
  CGFloat scale = _servoContentView.window.backingScaleFactor
      ?: NSScreen.mainScreen.backingScaleFactor
      ?: 1.0;
  NSSize size = _servoContentView.bounds.size;
  double width = std::max<CGFloat>(0, size.width) * scale;
  double height = std::max<CGFloat>(0, size.height) * scale;
  ServokitDesktopPrivateViewport viewport = {};
  viewport.width = static_cast<uint32_t>(
      std::min<double>(std::llround(width), std::numeric_limits<uint32_t>::max()));
  viewport.height = static_cast<uint32_t>(
      std::min<double>(std::llround(height), std::numeric_limits<uint32_t>::max()));
  viewport.scale_factor = scale;
  return viewport;
}

- (void)detachSurface
{
  if (_host == nullptr || _attachmentGeneration == 0) {
    return;
  }
  ServokitDesktopPrivateStatus status =
      servokit_desktop_private_detach(_host, _token);
  if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
    _attachmentGeneration = 0;
  } else if (status == SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR ||
             status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
      status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
    [self hostWasConsumed];
  } else if (status == SERVOKIT_DESKTOP_PRIVATE_BUSY) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [self syncSurface];
    });
  }
}

- (void)syncSurface
{
  if (![NSThread isMainThread]) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [self syncSurface];
    });
    return;
  }

  ServokitDesktopPrivateViewport viewport = [self currentViewport];
  if (_servoContentView.window == nil || viewport.width == 0 || viewport.height == 0) {
    [self detachSurface];
    return;
  }
  if (![self createHostIfNeeded]) {
    return;
  }

  if (_attachmentGeneration == 0) {
    [self loadURLIfNeededAllowingDetached:YES];
    if (_host == nullptr) {
      return;
    }
    ServokitDesktopPrivateNativeSurface surface = {
        .window = (__bridge void *)_servoContentView,
        .display = nullptr,
    };
    uint64_t generation = 0;
    ServokitDesktopPrivateStatus status =
        servokit_desktop_private_attach(_host, _token, surface, viewport, &generation);
    if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
      _attachmentGeneration = generation;
      _viewport = viewport;
      [self pumpForToken:_token];
      [self loadURLIfNeeded];
    } else if (status == SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR ||
               status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
               status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
      [self hostWasConsumed];
    }
    return;
  }

  [self loadURLIfNeeded];
  if (_viewport.width == viewport.width && _viewport.height == viewport.height &&
      _viewport.scale_factor == viewport.scale_factor) {
    return;
  }

  ServokitDesktopPrivateStatus status =
      servokit_desktop_private_update_viewport(_host, _token, viewport);
  if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
    _viewport = viewport;
    [self pumpForToken:_token];
  } else if (status == SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR ||
             status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
             status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
    [self hostWasConsumed];
  }
}

- (void)updateLayoutMetrics:(const LayoutMetrics &)layoutMetrics
           oldLayoutMetrics:(const LayoutMetrics &)oldLayoutMetrics
{
  [super updateLayoutMetrics:layoutMetrics oldLayoutMetrics:oldLayoutMetrics];
  [self syncSurface];
}

- (void)updateProps:(Props::Shared const &)props oldProps:(Props::Shared const &)oldProps
{
  [super updateProps:props oldProps:oldProps];
  const auto &viewProps = *std::static_pointer_cast<const ServoViewProps>(props);
  _url = [NSString stringWithUTF8String:viewProps.url.c_str()] ?: @"";
  [self syncSurface];
}

- (void)handleCommand:(const NSString *)commandName args:(const NSArray *)args
{
  RCTServoViewHandleCommand(self, commandName, args);
}

- (void)sendControllerCommand:(NSString *)commandJson
{
  [self dispatchControllerCommand:commandJson];
}

- (ServokitDesktopPrivateStatus)dispatchControllerCommand:(NSString *)commandJson
{
  if (![self createHostIfNeeded] || commandJson.length == 0) {
    return SERVOKIT_DESKTOP_PRIVATE_INVALID_ARGUMENT;
  }
  NSData *bytes = [commandJson dataUsingEncoding:NSUTF8StringEncoding];
  if (bytes.length == 0) {
    return SERVOKIT_DESKTOP_PRIVATE_INVALID_ARGUMENT;
  }
  ServokitDesktopPrivateStatus status = servokit_desktop_private_dispatch_controller_command(
      _host,
      _token,
      static_cast<const uint8_t *>(bytes.bytes),
      bytes.length);
  if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
    [self pumpForToken:_token];
  } else if (status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
             status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
    [self hostWasConsumed];
  }
  return status;
}

- (void)loadURLIfNeeded
{
  [self loadURLIfNeededAllowingDetached:NO];
}

- (void)loadURLIfNeededAllowingDetached:(BOOL)allowDetached
{
  if ((!allowDetached && _attachmentGeneration == 0) ||
      _url == nil || [_loadedURL isEqualToString:_url] ||
      ![self createHostIfNeeded]) {
    return;
  }
  NSData *data = [NSJSONSerialization dataWithJSONObject:@{
    @"version" : @1,
    @"command" : @"loadUrl",
    @"url" : _url,
  } options:0 error:nil];
  NSString *command = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  if ([self dispatchControllerCommand:command] == SERVOKIT_DESKTOP_PRIVATE_OK) {
    _loadedURL = [_url copy];
  }
}

- (void)dispatchInput:(ServokitDesktopPrivateInput)input
{
  if (_host == nullptr || _attachmentGeneration == 0) {
    return;
  }
  ServokitDesktopPrivateStatus status =
      servokit_desktop_private_dispatch_input(_host, _token, input);
  if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
    [self pumpForToken:_token];
  } else if (status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
             status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
    [self hostWasConsumed];
  }
}

- (void)pumpForToken:(uint64_t)token
{
  if (![NSThread isMainThread]) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [self pumpForToken:token];
    });
    return;
  }
  if (_host == nullptr || token == 0 || token != _token || _isPumping) {
    return;
  }

  _isPumping = YES;
  ServokitDesktopPrivateStatus status =
      servokit_desktop_private_pump(_host, _token);
  if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
    status = servokit_desktop_private_drain_events(
        _host, _token, ServoKitEvent, (__bridge void *)self);
  }
  _isPumping = NO;
  if (status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
      status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
    [self hostWasConsumed];
  }
}

- (void)prepareForRecycle
{
  [super prepareForRecycle];
  [_servoContentView resetTextInput];
  _url = nil;
  _wakeContext.view = nil;
  _servoContentView.servoView = nil;
  [self destroyHost];
  [self installAdapterLifetimes];
}

- (void)setCursorForName:(NSString *)name
{
  NSCursor *cursor = NSCursor.arrowCursor;
  if ([name isEqualToString:@"pointer"]) {
    cursor = NSCursor.pointingHandCursor;
  } else if ([name isEqualToString:@"text"] || [name isEqualToString:@"vertical-text"]) {
    cursor = NSCursor.IBeamCursor;
  } else if ([name isEqualToString:@"crosshair"]) {
    cursor = NSCursor.crosshairCursor;
  } else if ([name isEqualToString:@"grab"]) {
    cursor = NSCursor.openHandCursor;
  } else if ([name isEqualToString:@"grabbing"]) {
    cursor = NSCursor.closedHandCursor;
  } else if ([name containsString:@"ew-resize"] || [name isEqualToString:@"e-resize"] ||
             [name isEqualToString:@"w-resize"] || [name isEqualToString:@"col-resize"]) {
    cursor = NSCursor.resizeLeftRightCursor;
  } else if ([name containsString:@"ns-resize"] || [name isEqualToString:@"n-resize"] ||
             [name isEqualToString:@"s-resize"] || [name isEqualToString:@"row-resize"]) {
    cursor = NSCursor.resizeUpDownCursor;
  } else if ([name isEqualToString:@"not-allowed"] || [name isEqualToString:@"no-drop"]) {
    cursor = NSCursor.operationNotAllowedCursor;
  }
  [cursor set];
}

- (void)receiveEventJson:(const char *)eventJson
                   token:(uint64_t)token
      attachmentGeneration:(uint64_t)attachmentGeneration
{
  if (eventJson == nullptr || token != _token ||
      (attachmentGeneration != 0 && attachmentGeneration != _attachmentGeneration)) {
    return;
  }

  NSData *data = [NSData dataWithBytes:eventJson length:strlen(eventJson)];
  id decoded = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
  NSDictionary<NSString *, id> *event = ServoKitDictionary(decoded);
  NSString *name = ServoKitString(event[@"name"]);
  NSDictionary<NSString *, id> *payload = ServoKitDictionary(event[@"payload"]);
  if (name == nil || payload == nil || _eventEmitter == nullptr) {
    return;
  }

  auto emitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  if ([name isEqualToString:@"urlChanged"]) {
    emitter->onUrlChanged({.url = ServoKitStdString(payload[@"url"])});
  } else if ([name isEqualToString:@"pageTitleChanged"]) {
    emitter->onPageTitleChanged({.title = ServoKitStdString(payload[@"title"])});
  } else if ([name isEqualToString:@"statusTextChanged"]) {
    emitter->onStatusTextChanged({.status = ServoKitStdString(payload[@"status"])});
  } else if ([name isEqualToString:@"loadStatusChanged"]) {
    emitter->onLoadStatusChanged({
        .status = ServoKitStdString(payload[@"status"]),
    });
  } else if ([name isEqualToString:@"historyChanged"]) {
    std::vector<std::string> entries;
    for (id entry in ServoKitArray(payload[@"entries"])) {
      if ([entry isKindOfClass:NSString.class]) {
        entries.push_back(ServoKitStdString(entry));
      }
    }
    emitter->onHistoryChanged({
        .entries = entries,
        .current = ServoKitInt(payload[@"current"]),
        .canGoBack = static_cast<bool>(ServoKitBool(payload[@"canGoBack"])),
        .canGoForward = static_cast<bool>(ServoKitBool(payload[@"canGoForward"])),
    });
  } else if ([name isEqualToString:@"focusChanged"]) {
    emitter->onFocusChanged({
        .isFocused = static_cast<bool>(ServoKitBool(payload[@"isFocused"])),
    });
  } else if ([name isEqualToString:@"cursorChanged"]) {
    NSString *cursor = ServoKitString(payload[@"cursor"]) ?: @"default";
    [self setCursorForName:cursor];
    emitter->onCursorChanged({.cursor = ServoKitStdString(cursor)});
  } else if ([name isEqualToString:@"fullscreenChanged"]) {
    emitter->onFullscreenChanged({
        .isFullscreen = static_cast<bool>(ServoKitBool(payload[@"isFullscreen"])),
    });
  } else if ([name isEqualToString:@"closed"]) {
    emitter->onClosed({});
  } else if ([name isEqualToString:@"crashed"]) {
    emitter->onCrashed({
        .reason = ServoKitStdString(payload[@"reason"]),
        .backtrace = ServoKitStdString(payload[@"backtrace"]),
    });
  } else if ([name isEqualToString:@"error"]) {
    emitter->onError({
        .code = ServoKitInt(payload[@"code"]),
        .message = ServoKitStdString(payload[@"message"]),
    });
  } else if ([name isEqualToString:@"javascriptEvaluationResult"]) {
    emitter->onJavaScriptEvaluationResult({
        .evaluationId = ServoKitStdString(payload[@"evaluationId"]),
        .ok = static_cast<bool>(ServoKitBool(payload[@"ok"])),
        .valueJson = ServoKitStdString(payload[@"valueJson"]),
        .errorType = ServoKitStdString(payload[@"errorType"]),
    });
  }
}

@end

extern "C" Class<RCTComponentViewProtocol> ServoViewCls(void)
{
  return ServoView.class;
}
