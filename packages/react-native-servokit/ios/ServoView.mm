#include <cmath>
#include <memory>
#include <string>
#include <vector>

#import <Foundation/Foundation.h>
#import <ServoKitController.h>
#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>

#ifdef RCT_NEW_ARCH_ENABLED
#import <React/RCTViewComponentView.h>
#import <react/renderer/components/ServoViewSpec/ComponentDescriptors.h>
#import <react/renderer/components/ServoViewSpec/EventEmitters.h>
#import <react/renderer/components/ServoViewSpec/Props.h>
#import <react/renderer/components/ServoViewSpec/RCTComponentViewHelpers.h>
#else
#import <React/RCTViewManager.h>
#endif

static NSString *const ServoKitLoadStatusStarted = @"Started";
static NSString *const ServoKitLoadStatusHeadParsed = @"HeadParsed";
static NSString *const ServoKitLoadStatusComplete = @"Complete";
static NSString *const ServoKitJavaScriptEvaluationSerializationError = @"SerializationError";
static NSString *const ServoKitJavaScriptEvaluationFailure = @"EvaluationFailure";
static NSString *const ServoKitJavaScriptEvaluationInternalError = @"InternalError";
static NSString *const ServoKitJavaScriptDialogKindAlert = @"alert";
static NSString *const ServoKitJavaScriptDialogKindConfirm = @"confirm";
static NSString *const ServoKitJavaScriptDialogKindPrompt = @"prompt";

typedef void (^ServoKitNavigationDecisionHandler)(WKNavigationActionPolicy policy);
typedef void (^ServoKitJavaScriptDialogResolutionHandler)(BOOL confirmed, NSString *_Nullable promptValue);

static NSURL *ServoKitURLFromString(NSString *urlString)
{
  if (urlString.length == 0) {
    return nil;
  }

  NSURL *url = [NSURL URLWithString:urlString];
  if (url != nil && url.scheme.length > 0) {
    return url;
  }

  NSString *escaped = [urlString stringByAddingPercentEncodingWithAllowedCharacters:NSCharacterSet.URLQueryAllowedCharacterSet];
  return escaped.length > 0 ? [NSURL URLWithString:escaped] : nil;
}

static NSString *ServoKitURLString(NSURL *url)
{
  return url.absoluteString ?: @"";
}

static std::string ServoKitStdString(NSString *value)
{
  return std::string(value.UTF8String ?: "");
}

static NSDictionary<NSString *, id> *ServoKitTaggedValue(NSString *type)
{
  return @{@"type" : type};
}

static NSDictionary<NSString *, id> *ServoKitTaggedScalarValue(NSString *type, id value)
{
  return @{@"type" : type, @"value" : value};
}

static id ServoKitSerializeJavaScriptValue(id value, BOOL *ok)
{
  if (value == nil || value == (id)kCFNull || [value isKindOfClass:NSNull.class]) {
    return ServoKitTaggedValue(@"null");
  }

  if ([value isKindOfClass:NSString.class]) {
    return ServoKitTaggedScalarValue(@"string", value);
  }

  if ([value isKindOfClass:NSNumber.class]) {
    if (CFGetTypeID((__bridge CFTypeRef)value) == CFBooleanGetTypeID()) {
      return ServoKitTaggedScalarValue(@"boolean", value);
    }

    double number = [(NSNumber *)value doubleValue];
    if (!std::isfinite(number)) {
      *ok = NO;
      return nil;
    }
    return ServoKitTaggedScalarValue(@"number", value);
  }

  if ([value isKindOfClass:NSArray.class]) {
    NSMutableArray *items = [NSMutableArray array];
    for (id item in (NSArray *)value) {
      id serializedItem = ServoKitSerializeJavaScriptValue(item, ok);
      if (!*ok) {
        return nil;
      }
      [items addObject:serializedItem];
    }
    return ServoKitTaggedScalarValue(@"array", items);
  }

  if ([value isKindOfClass:NSDictionary.class]) {
    NSMutableDictionary<NSString *, id> *object = [NSMutableDictionary dictionary];
    for (id key in (NSDictionary *)value) {
      if (![key isKindOfClass:NSString.class]) {
        *ok = NO;
        return nil;
      }

      id serializedValue = ServoKitSerializeJavaScriptValue(((NSDictionary *)value)[key], ok);
      if (!*ok) {
        return nil;
      }
      object[(NSString *)key] = serializedValue;
    }
    return ServoKitTaggedScalarValue(@"object", object);
  }

  *ok = NO;
  return nil;
}

static NSString *ServoKitJavaScriptValueJson(id value)
{
  BOOL ok = YES;
  id serializedValue = ServoKitSerializeJavaScriptValue(value, &ok);
  if (!ok || serializedValue == nil) {
    return nil;
  }

  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:serializedValue options:NSJSONWritingSortedKeys error:&error];
  if (data == nil || error != nil) {
    return nil;
  }

  return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
}

static NSDictionary<NSString *, id> *ServoKitCopyControllerOutput(
    ServoKitControllerResult result,
    uint32_t *status,
    uint64_t *handle)
{
  if (status != nullptr) {
    *status = result.status;
  }
  if (handle != nullptr) {
    *handle = result.handle;
  }

  NSData *data = nil;
  if (result.bytes != nullptr && result.len > 0) {
    data = [NSData dataWithBytes:result.bytes length:result.len];
  }
  BOOL validBuffer = (result.bytes == nullptr && result.len == 0) || data != nil;
  servokit_controller_result_free(result);
  if (!validBuffer || data.length == 0) {
    return nil;
  }

  NSError *error = nil;
  id output = [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
  if (error != nil || ![output isKindOfClass:NSDictionary.class]) {
    return nil;
  }
  return output;
}

@interface ServoKitWKWebView : UIView <WKNavigationDelegate, WKUIDelegate>
@property (nonatomic, copy, nullable) NSString *url;
@property (nonatomic, copy, nullable) void (^onUrlChanged)(NSString *url);
@property (nonatomic, copy, nullable) void (^onPageTitleChanged)(NSString *_Nullable title);
@property (nonatomic, copy, nullable) void (^onLoadStatusChanged)(NSString *url, NSString *status);
@property (nonatomic, assign) BOOL useReactNativeJavaScriptDialogs;
@property (nonatomic, assign) BOOL useReactNativeOnShouldStartLoadWithRequest;
@property (nonatomic, copy, nullable) void (^onHistoryChanged)(NSArray<NSString *> *entries, NSInteger current, BOOL canGoBack, BOOL canGoForward);
@property (nonatomic, copy, nullable) void (^onFocusChanged)(BOOL isFocused);
@property (nonatomic, copy, nullable) void (^onJavaScriptEvaluationResult)(NSString *evaluationId, BOOL ok, NSString *_Nullable valueJson, NSString *_Nullable errorType);
@property (nonatomic, copy, nullable) BOOL (^onJavaScriptDialogRequested)(NSString *dialogId, NSString *kind, NSString *message, NSString *_Nullable defaultValue);
@property (nonatomic, copy, nullable) void (^onJavaScriptDialogDismissed)(NSString *dialogId);
@property (nonatomic, copy, nullable) BOOL (^onShouldStartLoadWithRequestRequested)(NSString *navigationId, NSString *url);
- (void)sendCommandJson:(NSString *)commandJson;
- (void)prepareForReuse;
@end

@interface ServoKitWKWebView ()
- (BOOL)createController;
- (void)replaceWebView;
@end

@implementation ServoKitWKWebView {
  uint64_t _controllerHandle;
  uint64_t _controllerGeneration;
  WKWebView *_webView;
  NSString *_rawURLProp;
  NSString *_loadedURL;
  NSMutableDictionary<NSString *, NSString *> *_pendingJavaScriptEvaluationIdsByRequestId;
  NSMutableDictionary<NSString *, ServoKitJavaScriptDialogResolutionHandler> *_pendingJavaScriptDialogHandlers;
  NSMutableDictionary<NSString *, ServoKitNavigationDecisionHandler> *_pendingNavigationDecisionHandlers;
  NSMutableDictionary<NSString *, NSTimer *> *_pendingControllerTimers;
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if ((self = [super initWithFrame:frame])) {
    self.backgroundColor = UIColor.clearColor;
    self.userInteractionEnabled = YES;
    _pendingJavaScriptEvaluationIdsByRequestId = [NSMutableDictionary dictionary];
    _pendingJavaScriptDialogHandlers = [NSMutableDictionary dictionary];
    _pendingNavigationDecisionHandlers = [NSMutableDictionary dictionary];
    _pendingControllerTimers = [NSMutableDictionary dictionary];
    [self createController];
    [self replaceWebView];
  }
  return self;
}

- (BOOL)createController
{
  if (_controllerHandle != 0) {
    return NO;
  }

  ServoKitControllerResult result = servokit_controller_create();
  uint32_t status = SERVOKIT_CONTROLLER_INTERNAL_ERROR;
  uint64_t handle = 0;
  NSDictionary *output = ServoKitCopyControllerOutput(result, &status, &handle);
  NSNumber *version = [output[@"version"] isKindOfClass:NSNumber.class] ? output[@"version"] : nil;
  NSArray *effects = [output[@"effects"] isKindOfClass:NSArray.class] ? output[@"effects"] : nil;
  NSArray *events = [output[@"events"] isKindOfClass:NSArray.class] ? output[@"events"] : nil;
  if (status == SERVOKIT_CONTROLLER_OK && handle != 0 && version.integerValue == 1 && effects != nil && events != nil &&
      effects.count == 0 && events.count == 0) {
    _controllerHandle = handle;
    _controllerGeneration = 1;
    return YES;
  }

  if (handle != 0) {
    ServoKitControllerResult destroyResult = servokit_controller_destroy(handle);
    ServoKitCopyControllerOutput(destroyResult, nullptr, nullptr);
  }
  _controllerGeneration = 0;
  return NO;
}

- (void)replaceWebView
{
  if (_webView != nil) {
    _webView.UIDelegate = nil;
    _webView.navigationDelegate = nil;
    [_webView removeObserver:self forKeyPath:@"title"];
    [_webView stopLoading];
    [_webView removeFromSuperview];
  }

  _webView = [[WKWebView alloc] initWithFrame:self.bounds];
  _webView.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
  _webView.navigationDelegate = self;
  _webView.UIDelegate = self;
  _webView.opaque = NO;
  _webView.backgroundColor = UIColor.clearColor;
  [_webView addObserver:self forKeyPath:@"title" options:NSKeyValueObservingOptionNew context:nil];
  [self addSubview:_webView];
  _rawURLProp = nil;
  _url = nil;
  _loadedURL = nil;
}

- (void)dealloc
{
  if (_controllerHandle != 0) {
    ServoKitControllerResult result = servokit_controller_destroy(_controllerHandle);
    _controllerHandle = 0;
    uint32_t status = SERVOKIT_CONTROLLER_INTERNAL_ERROR;
    uint64_t returnedHandle = UINT64_MAX;
    NSDictionary *output = ServoKitCopyControllerOutput(result, &status, &returnedHandle);
    BOOL retainedNavigation = NO;
    BOOL retainedDialog = NO;
    if (status != SERVOKIT_CONTROLLER_OK || returnedHandle != 0 || output == nil ||
        ![self applyControllerOutput:output
                navigationCompletion:nil
                    dialogCompletion:nil
                   retainedNavigation:&retainedNavigation
                       retainedDialog:&retainedDialog]) {
      [self failNativeCompletions];
    }
  } else {
    [self failNativeCompletions];
  }
  _webView.UIDelegate = nil;
  _webView.navigationDelegate = nil;
  [_webView removeObserver:self forKeyPath:@"title"];
}

- (void)invalidateControllerTimer:(NSString *)requestId
{
  NSTimer *timer = _pendingControllerTimers[requestId];
  [timer invalidate];
  [_pendingControllerTimers removeObjectForKey:requestId];
}

- (void)emitJavaScriptEvaluationInternalError:(NSString *)evaluationId
{
  if (evaluationId.length > 0 && self.onJavaScriptEvaluationResult != nil) {
    self.onJavaScriptEvaluationResult(evaluationId, NO, nil, ServoKitJavaScriptEvaluationInternalError);
  }
}

- (void)failNativeCompletionsWithEvaluationId:(NSString *_Nullable)additionalEvaluationId
{
  NSArray<ServoKitNavigationDecisionHandler> *navigationHandlers = _pendingNavigationDecisionHandlers.allValues;
  NSDictionary<NSString *, ServoKitJavaScriptDialogResolutionHandler> *dialogHandlers = [_pendingJavaScriptDialogHandlers copy];
  NSMutableOrderedSet<NSString *> *evaluationIds =
      [NSMutableOrderedSet orderedSetWithArray:_pendingJavaScriptEvaluationIdsByRequestId.allValues];
  if (additionalEvaluationId.length > 0) {
    [evaluationIds addObject:additionalEvaluationId];
  }

  for (NSTimer *timer in _pendingControllerTimers.allValues) {
    [timer invalidate];
  }
  [_pendingControllerTimers removeAllObjects];
  [_pendingNavigationDecisionHandlers removeAllObjects];
  [_pendingJavaScriptDialogHandlers removeAllObjects];
  [_pendingJavaScriptEvaluationIdsByRequestId removeAllObjects];

  for (ServoKitNavigationDecisionHandler handler in navigationHandlers) {
    handler(WKNavigationActionPolicyAllow);
  }
  for (NSString *dialogId in dialogHandlers) {
    ServoKitJavaScriptDialogResolutionHandler handler = dialogHandlers[dialogId];
    handler(NO, nil);
    if (self.onJavaScriptDialogDismissed != nil) {
      self.onJavaScriptDialogDismissed(dialogId);
    }
  }
  for (NSString *evaluationId in evaluationIds) {
    [self emitJavaScriptEvaluationInternalError:evaluationId];
  }
}

- (void)failNativeCompletions
{
  [self failNativeCompletionsWithEvaluationId:nil];
}

- (void)retireControllerAfterFailure:(NSString *_Nullable)evaluationId
{
  uint64_t handle = _controllerHandle;
  _controllerHandle = 0;
  _controllerGeneration = 0;
  if (handle != 0) {
    ServoKitControllerResult result = servokit_controller_destroy(handle);
    ServoKitCopyControllerOutput(result, nullptr, nullptr);
  }
  [self createController];
  [self failNativeCompletionsWithEvaluationId:evaluationId];
}

- (BOOL)dispatchControllerData:(NSData *)data
          navigationCompletion:(ServoKitNavigationDecisionHandler _Nullable)navigationCompletion
              dialogCompletion:(ServoKitJavaScriptDialogResolutionHandler _Nullable)dialogCompletion
            evaluationIdOnError:(NSString *_Nullable)evaluationIdOnError
{
  if (data.length == 0 || (_controllerHandle == 0 && ![self createController])) {
    if (navigationCompletion != nil) {
      navigationCompletion(WKNavigationActionPolicyAllow);
    }
    if (dialogCompletion != nil) {
      dialogCompletion(NO, nil);
    }
    [self emitJavaScriptEvaluationInternalError:evaluationIdOnError];
    return NO;
  }

  uint64_t activeHandle = _controllerHandle;
  ServoKitControllerResult result = servokit_controller_dispatch(
      activeHandle,
      static_cast<const uint8_t *>(data.bytes),
      data.length);
  uint32_t status = SERVOKIT_CONTROLLER_INTERNAL_ERROR;
  uint64_t returnedHandle = 0;
  NSDictionary *output = ServoKitCopyControllerOutput(result, &status, &returnedHandle);
  if (returnedHandle == 0) {
    _controllerHandle = 0;
  }
  if (status != SERVOKIT_CONTROLLER_OK || returnedHandle != activeHandle || output == nil) {
    BOOL controllerRetired = returnedHandle == 0 || returnedHandle != activeHandle || output == nil;
    if (controllerRetired) {
      [self retireControllerAfterFailure:evaluationIdOnError];
    } else {
      [self emitJavaScriptEvaluationInternalError:evaluationIdOnError];
    }
    if (navigationCompletion != nil) {
      navigationCompletion(WKNavigationActionPolicyAllow);
    }
    if (dialogCompletion != nil) {
      dialogCompletion(NO, nil);
    }
    return NO;
  }

  BOOL retainedNavigation = NO;
  BOOL retainedDialog = NO;
  if (![self applyControllerOutput:output
              navigationCompletion:navigationCompletion
                  dialogCompletion:dialogCompletion
                 retainedNavigation:&retainedNavigation
                     retainedDialog:&retainedDialog]) {
    [self retireControllerAfterFailure:evaluationIdOnError];
    if (navigationCompletion != nil && !retainedNavigation) {
      navigationCompletion(WKNavigationActionPolicyAllow);
    }
    if (dialogCompletion != nil && !retainedDialog) {
      dialogCompletion(NO, nil);
    }
    return NO;
  }
  return YES;
}

- (BOOL)dispatchControllerEnvelope:(NSDictionary<NSString *, id> *)envelope
              navigationCompletion:(ServoKitNavigationDecisionHandler _Nullable)navigationCompletion
                  dialogCompletion:(ServoKitJavaScriptDialogResolutionHandler _Nullable)dialogCompletion
{
  if (![NSJSONSerialization isValidJSONObject:envelope]) {
    if (navigationCompletion != nil) {
      navigationCompletion(WKNavigationActionPolicyAllow);
    }
    if (dialogCompletion != nil) {
      dialogCompletion(NO, nil);
    }
    return NO;
  }
  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:envelope options:0 error:&error];
  if (error != nil || data == nil) {
    if (navigationCompletion != nil) {
      navigationCompletion(WKNavigationActionPolicyAllow);
    }
    if (dialogCompletion != nil) {
      dialogCompletion(NO, nil);
    }
    return NO;
  }
  return [self dispatchControllerData:data
                 navigationCompletion:navigationCompletion
                     dialogCompletion:dialogCompletion
                   evaluationIdOnError:nil];
}

- (BOOL)dispatchControllerEnvelope:(NSDictionary<NSString *, id> *)envelope
{
  return [self dispatchControllerEnvelope:envelope navigationCompletion:nil dialogCompletion:nil];
}

- (BOOL)applyControllerOutput:(NSDictionary<NSString *, id> *)output
         navigationCompletion:(ServoKitNavigationDecisionHandler _Nullable)navigationCompletion
             dialogCompletion:(ServoKitJavaScriptDialogResolutionHandler _Nullable)dialogCompletion
            retainedNavigation:(BOOL *)retainedNavigation
                retainedDialog:(BOOL *)retainedDialog
{
  NSNumber *version = [output[@"version"] isKindOfClass:NSNumber.class] ? output[@"version"] : nil;
  NSArray *effects = [output[@"effects"] isKindOfClass:NSArray.class] ? output[@"effects"] : nil;
  NSArray *events = [output[@"events"] isKindOfClass:NSArray.class] ? output[@"events"] : nil;
  if (version.integerValue != 1 || effects == nil || events == nil) {
    return NO;
  }

  for (id value in effects) {
    if (![value isKindOfClass:NSDictionary.class] ||
        ![self executeControllerEffect:value
                  navigationCompletion:navigationCompletion
                      dialogCompletion:dialogCompletion
                     retainedNavigation:retainedNavigation
                         retainedDialog:retainedDialog]) {
      return NO;
    }
  }
  for (id value in events) {
    if (![value isKindOfClass:NSDictionary.class] || ![self emitControllerEvent:value]) {
      return NO;
    }
  }
  return YES;
}

- (BOOL)executeControllerEffect:(NSDictionary<NSString *, id> *)effect
           navigationCompletion:(ServoKitNavigationDecisionHandler _Nullable)navigationCompletion
               dialogCompletion:(ServoKitJavaScriptDialogResolutionHandler _Nullable)dialogCompletion
              retainedNavigation:(BOOL *)retainedNavigation
                  retainedDialog:(BOOL *)retainedDialog
{
  NSString *name = [effect[@"name"] isKindOfClass:NSString.class] ? effect[@"name"] : nil;
  if ([name isEqualToString:@"loadUrl"]) {
    NSString *url = [effect[@"url"] isKindOfClass:NSString.class] ? effect[@"url"] : nil;
    if (url == nil) {
      return NO;
    }
    [self loadURLString:url];
  } else if ([name isEqualToString:@"reload"]) {
    [_webView reload];
  } else if ([name isEqualToString:@"goBack"]) {
    [_webView goBack];
  } else if ([name isEqualToString:@"goForward"]) {
    [_webView goForward];
  } else if ([name isEqualToString:@"focus"]) {
    [_webView becomeFirstResponder];
    return [self dispatchControllerEnvelope:@{@"version" : @1, @"observation" : @"focusChanged", @"isFocused" : @YES}];
  } else if ([name isEqualToString:@"blur"]) {
    [_webView resignFirstResponder];
    return [self dispatchControllerEnvelope:@{@"version" : @1, @"observation" : @"focusChanged", @"isFocused" : @NO}];
  } else if ([name isEqualToString:@"evaluateJavaScript"]) {
    NSString *requestId = [effect[@"requestId"] isKindOfClass:NSString.class] ? effect[@"requestId"] : nil;
    NSString *evaluationId = [effect[@"evaluationId"] isKindOfClass:NSString.class] ? effect[@"evaluationId"] : nil;
    NSString *script = [effect[@"script"] isKindOfClass:NSString.class] ? effect[@"script"] : nil;
    if (requestId.length == 0 || evaluationId.length == 0 || script == nil ||
        _pendingJavaScriptEvaluationIdsByRequestId[requestId] != nil ||
        [_pendingJavaScriptEvaluationIdsByRequestId.allValues containsObject:evaluationId]) {
      return NO;
    }
    _pendingJavaScriptEvaluationIdsByRequestId[requestId] = evaluationId;
    __weak __typeof(self) weakSelf = self;
    [_webView evaluateJavaScript:script completionHandler:^(id result, NSError *error) {
      [weakSelf completeJavaScriptEvaluationRequest:requestId result:result error:error];
    }];
  } else if ([name isEqualToString:@"retainNavigationCompletion"]) {
    NSString *navigationId = [effect[@"navigationId"] isKindOfClass:NSString.class] ? effect[@"navigationId"] : nil;
    if (navigationId.length == 0 || navigationCompletion == nil || _pendingNavigationDecisionHandlers[navigationId] != nil) {
      return NO;
    }
    _pendingNavigationDecisionHandlers[navigationId] = [navigationCompletion copy];
    *retainedNavigation = YES;
  } else if ([name isEqualToString:@"retainDialogCompletion"]) {
    NSString *dialogId = [effect[@"dialogId"] isKindOfClass:NSString.class] ? effect[@"dialogId"] : nil;
    if (dialogId.length == 0 || dialogCompletion == nil || _pendingJavaScriptDialogHandlers[dialogId] != nil) {
      return NO;
    }
    _pendingJavaScriptDialogHandlers[dialogId] = [dialogCompletion copy];
    *retainedDialog = YES;
  } else if ([name isEqualToString:@"scheduleTimeout"]) {
    NSString *requestId = [effect[@"requestId"] isKindOfClass:NSString.class] ? effect[@"requestId"] : nil;
    NSString *kind = [effect[@"kind"] isKindOfClass:NSString.class] ? effect[@"kind"] : nil;
    NSNumber *timeoutMs = [effect[@"timeoutMs"] isKindOfClass:NSNumber.class] ? effect[@"timeoutMs"] : nil;
    NSTimeInterval timeoutSeconds = timeoutMs.doubleValue / 1000.0;
    BOOL hasCompletion = ([kind isEqualToString:@"navigation"] && _pendingNavigationDecisionHandlers[requestId] != nil) ||
        ([kind isEqualToString:@"dialog"] && _pendingJavaScriptDialogHandlers[requestId] != nil);
    if (requestId.length == 0 || !hasCompletion || !std::isfinite(timeoutSeconds) || timeoutSeconds <= 0 ||
        _pendingControllerTimers[requestId] != nil) {
      return NO;
    }
    __weak __typeof(self) weakSelf = self;
    NSTimer *timer = [NSTimer scheduledTimerWithTimeInterval:timeoutSeconds repeats:NO block:^(__unused NSTimer *firedTimer) {
      [weakSelf dispatchControllerEnvelope:@{
        @"version" : @1,
        @"observation" : @"nativeTimeout",
        @"requestId" : requestId,
        @"kind" : kind,
      }];
    }];
    _pendingControllerTimers[requestId] = timer;
  } else if ([name isEqualToString:@"resolveNavigationCompletion"]) {
    NSString *navigationId = [effect[@"navigationId"] isKindOfClass:NSString.class] ? effect[@"navigationId"] : nil;
    NSNumber *allow = [effect[@"allow"] isKindOfClass:NSNumber.class] ? effect[@"allow"] : nil;
    ServoKitNavigationDecisionHandler handler = _pendingNavigationDecisionHandlers[navigationId];
    if (navigationId.length == 0 || allow == nil || handler == nil) {
      return NO;
    }
    [self invalidateControllerTimer:navigationId];
    [_pendingNavigationDecisionHandlers removeObjectForKey:navigationId];
    handler(allow.boolValue ? WKNavigationActionPolicyAllow : WKNavigationActionPolicyCancel);
  } else if ([name isEqualToString:@"resolveDialogCompletion"]) {
    NSString *dialogId = [effect[@"dialogId"] isKindOfClass:NSString.class] ? effect[@"dialogId"] : nil;
    NSNumber *confirmed = [effect[@"confirmed"] isKindOfClass:NSNumber.class] ? effect[@"confirmed"] : nil;
    id promptValueField = effect[@"promptValue"];
    NSString *promptValue = [promptValueField isKindOfClass:NSString.class] ? promptValueField : nil;
    BOOL validPromptValue = promptValueField == nil || promptValueField == NSNull.null || promptValue != nil;
    ServoKitJavaScriptDialogResolutionHandler handler = _pendingJavaScriptDialogHandlers[dialogId];
    if (dialogId.length == 0 || confirmed == nil || !validPromptValue || handler == nil) {
      return NO;
    }
    [self invalidateControllerTimer:dialogId];
    [_pendingJavaScriptDialogHandlers removeObjectForKey:dialogId];
    handler(confirmed.boolValue, promptValue);
  } else if ([name isEqualToString:@"invalidateEvaluation"]) {
    NSString *requestId = [effect[@"requestId"] isKindOfClass:NSString.class] ? effect[@"requestId"] : nil;
    if (requestId.length == 0 || _pendingJavaScriptEvaluationIdsByRequestId[requestId] == nil) {
      return NO;
    }
  } else if ([name isEqualToString:@"reset"]) {
    NSNumber *generation = [effect[@"generation"] isKindOfClass:NSNumber.class] ? effect[@"generation"] : nil;
    uint64_t nextGeneration = generation.unsignedLongLongValue;
    if (generation == nil || _controllerGeneration == 0 || _controllerGeneration == UINT64_MAX ||
        nextGeneration != _controllerGeneration + 1 ||
        generation.doubleValue != static_cast<double>(nextGeneration)) {
      return NO;
    }
    _controllerGeneration = nextGeneration;
    [self replaceWebView];
  } else if ([name isEqualToString:@"destroy"]) {
    [_webView stopLoading];
  } else {
    return NO;
  }
  return YES;
}

- (BOOL)emitControllerEvent:(NSDictionary<NSString *, id> *)event
{
  NSString *name = [event[@"name"] isKindOfClass:NSString.class] ? event[@"name"] : nil;
  NSDictionary *payload = [event[@"payload"] isKindOfClass:NSDictionary.class] ? event[@"payload"] : nil;
  if (name == nil || payload == nil) {
    return NO;
  }

  if ([name isEqualToString:@"urlChanged"]) {
    NSString *url = [payload[@"url"] isKindOfClass:NSString.class] ? payload[@"url"] : nil;
    if (url == nil) {
      return NO;
    }
    if (self.onUrlChanged != nil) {
      self.onUrlChanged(url);
    }
  } else if ([name isEqualToString:@"pageTitleChanged"]) {
    id titleField = payload[@"title"];
    NSString *title = [titleField isKindOfClass:NSString.class] ? titleField : nil;
    if (titleField != NSNull.null && title == nil) {
      return NO;
    }
    if (self.onPageTitleChanged != nil) {
      self.onPageTitleChanged(title);
    }
  } else if ([name isEqualToString:@"loadStatusChanged"]) {
    NSString *status = [payload[@"status"] isKindOfClass:NSString.class] ? payload[@"status"] : nil;
    if (status == nil) {
      return NO;
    }
    if (self.onLoadStatusChanged != nil) {
      self.onLoadStatusChanged([self currentNavigationURLString:nil], status);
    }
  } else if ([name isEqualToString:@"historyChanged"]) {
    NSArray *entries = [payload[@"entries"] isKindOfClass:NSArray.class] ? payload[@"entries"] : nil;
    NSNumber *current = [payload[@"current"] isKindOfClass:NSNumber.class] ? payload[@"current"] : nil;
    NSNumber *canGoBack = [payload[@"canGoBack"] isKindOfClass:NSNumber.class] ? payload[@"canGoBack"] : nil;
    NSNumber *canGoForward = [payload[@"canGoForward"] isKindOfClass:NSNumber.class] ? payload[@"canGoForward"] : nil;
    if (entries == nil || current == nil || canGoBack == nil || canGoForward == nil) {
      return NO;
    }
    for (id entry in entries) {
      if (![entry isKindOfClass:NSString.class]) {
        return NO;
      }
    }
    if (self.onHistoryChanged != nil) {
      self.onHistoryChanged(entries, current.integerValue, canGoBack.boolValue, canGoForward.boolValue);
    }
  } else if ([name isEqualToString:@"focusChanged"]) {
    NSNumber *isFocused = [payload[@"isFocused"] isKindOfClass:NSNumber.class] ? payload[@"isFocused"] : nil;
    if (isFocused == nil) {
      return NO;
    }
    if (self.onFocusChanged != nil) {
      self.onFocusChanged(isFocused.boolValue);
    }
  } else if ([name isEqualToString:@"javascriptEvaluationResult"]) {
    NSString *evaluationId = [payload[@"evaluationId"] isKindOfClass:NSString.class] ? payload[@"evaluationId"] : nil;
    NSNumber *ok = [payload[@"ok"] isKindOfClass:NSNumber.class] ? payload[@"ok"] : nil;
    id valueJsonField = payload[@"valueJson"];
    id errorTypeField = payload[@"errorType"];
    NSString *valueJson = [valueJsonField isKindOfClass:NSString.class] ? valueJsonField : nil;
    NSString *errorType = [errorTypeField isKindOfClass:NSString.class] ? errorTypeField : nil;
    NSString *requestId = [_pendingJavaScriptEvaluationIdsByRequestId allKeysForObject:evaluationId].firstObject;
    if (evaluationId.length == 0 || requestId == nil || ok == nil ||
        (valueJsonField != NSNull.null && valueJson == nil) ||
        (errorTypeField != NSNull.null && errorType == nil)) {
      return NO;
    }
    [_pendingJavaScriptEvaluationIdsByRequestId removeObjectForKey:requestId];
    if (self.onJavaScriptEvaluationResult != nil) {
      self.onJavaScriptEvaluationResult(evaluationId, ok.boolValue, valueJson, errorType);
    }
  } else if ([name isEqualToString:@"simpleDialogRequested"]) {
    NSString *dialogId = [payload[@"dialogId"] isKindOfClass:NSString.class] ? payload[@"dialogId"] : nil;
    NSString *kind = [payload[@"kind"] isKindOfClass:NSString.class] ? payload[@"kind"] : nil;
    NSString *message = [payload[@"message"] isKindOfClass:NSString.class] ? payload[@"message"] : nil;
    id defaultValueField = payload[@"defaultValue"];
    NSString *defaultValue = [defaultValueField isKindOfClass:NSString.class] ? defaultValueField : nil;
    if (dialogId == nil || kind == nil || message == nil ||
        (defaultValueField != NSNull.null && defaultValue == nil)) {
      return NO;
    }
    BOOL dispatched = self.useReactNativeJavaScriptDialogs && self.onJavaScriptDialogRequested != nil &&
        self.onJavaScriptDialogRequested(dialogId, kind, message, defaultValue);
    if (!dispatched) {
      return [self dispatchControllerEnvelope:@{
        @"version" : @1,
        @"observation" : @"nativeTimeout",
        @"requestId" : dialogId,
        @"kind" : @"dialog",
      }];
    }
  } else if ([name isEqualToString:@"simpleDialogDismissed"]) {
    NSString *dialogId = [payload[@"dialogId"] isKindOfClass:NSString.class] ? payload[@"dialogId"] : nil;
    if (dialogId == nil) {
      return NO;
    }
    if (self.onJavaScriptDialogDismissed != nil) {
      self.onJavaScriptDialogDismissed(dialogId);
    }
  } else if ([name isEqualToString:@"navigationRequested"]) {
    NSString *navigationId = [payload[@"navigationId"] isKindOfClass:NSString.class] ? payload[@"navigationId"] : nil;
    NSString *url = [payload[@"url"] isKindOfClass:NSString.class] ? payload[@"url"] : nil;
    if (navigationId == nil || url == nil) {
      return NO;
    }
    BOOL dispatched = self.useReactNativeOnShouldStartLoadWithRequest &&
        self.onShouldStartLoadWithRequestRequested != nil &&
        self.onShouldStartLoadWithRequestRequested(navigationId, url);
    if (!dispatched) {
      return [self dispatchControllerEnvelope:@{
        @"version" : @1,
        @"observation" : @"nativeTimeout",
        @"requestId" : navigationId,
        @"kind" : @"navigation",
      }];
    }
  } else {
    return NO;
  }
  return YES;
}

- (void)layoutSubviews
{
  [super layoutSubviews];
  _webView.frame = self.bounds;
}

- (void)setUrl:(NSString *)url
{
  NSString *nextURL = [url copy] ?: @"";
  if ([_rawURLProp isEqualToString:nextURL]) {
    return;
  }

  if ([self dispatchControllerEnvelope:@{
    @"version" : @1,
    @"command" : @"loadUrl",
    @"url" : nextURL,
  }]) {
    _rawURLProp = nextURL;
  }
}

- (void)sendCommandJson:(NSString *)commandJson
{
  if (commandJson.length == 0) {
    return;
  }

  NSData *data = [commandJson dataUsingEncoding:NSUTF8StringEncoding];
  if (data != nil) {
    NSError *error = nil;
    id value = [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
    NSDictionary *envelope = [value isKindOfClass:NSDictionary.class] ? value : nil;
    NSString *command = [envelope[@"command"] isKindOfClass:NSString.class] ? envelope[@"command"] : nil;
    NSString *evaluationId = [envelope[@"evaluationId"] isKindOfClass:NSString.class] ? envelope[@"evaluationId"] : nil;
    NSString *evaluationIdOnError = error == nil && [command isEqualToString:@"evaluateJavaScript"] ? evaluationId : nil;
    [self dispatchControllerData:data
           navigationCompletion:nil
               dialogCompletion:nil
             evaluationIdOnError:evaluationIdOnError];
  }
}

- (void)completeJavaScriptEvaluationRequest:(NSString *)requestId result:(id)result error:(NSError *)error
{
  if (_pendingJavaScriptEvaluationIdsByRequestId[requestId] == nil) {
    return;
  }

  NSString *valueJson = error == nil ? ServoKitJavaScriptValueJson(result) : nil;
  NSDictionary<NSString *, id> *observation;
  if (error != nil) {
    observation = @{
      @"version" : @1,
      @"observation" : @"javaScriptEvaluationResult",
      @"requestId" : requestId,
      @"ok" : @NO,
      @"errorType" : ServoKitJavaScriptEvaluationFailure,
    };
  } else if (valueJson != nil) {
    observation = @{
      @"version" : @1,
      @"observation" : @"javaScriptEvaluationResult",
      @"requestId" : requestId,
      @"ok" : @YES,
      @"valueJson" : valueJson,
    };
  } else {
    observation = @{
      @"version" : @1,
      @"observation" : @"javaScriptEvaluationResult",
      @"requestId" : requestId,
      @"ok" : @NO,
      @"errorType" : ServoKitJavaScriptEvaluationSerializationError,
    };
  }

  if (![self dispatchControllerEnvelope:observation] &&
      _pendingJavaScriptEvaluationIdsByRequestId[requestId] != nil) {
    [self retireControllerAfterFailure:nil];
  }
}

- (void)requestJavaScriptDialogWithKind:(NSString *)kind
                                message:(NSString *)message
                           defaultValue:(NSString *_Nullable)defaultValue
                      completionHandler:(ServoKitJavaScriptDialogResolutionHandler)completionHandler
{
  NSMutableDictionary<NSString *, id> *observation = [@{
    @"version" : @1,
    @"observation" : @"simpleDialogRequest",
    @"kind" : kind,
    @"message" : message ?: @"",
  } mutableCopy];
  if (defaultValue != nil) {
    observation[@"defaultValue"] = defaultValue;
  }
  [self dispatchControllerEnvelope:observation navigationCompletion:nil dialogCompletion:completionHandler];
}

- (void)prepareForReuse
{
  if (![self dispatchControllerEnvelope:@{@"version" : @1, @"lifecycle" : @"reset"}]) {
    [self replaceWebView];
  }
}

- (void)loadURLString:(NSString *)urlString
{
  NSString *nextURL = [urlString copy] ?: @"";
  NSURL *url = ServoKitURLFromString(nextURL);
  if (url == nil) {
    return;
  }

  _url = nextURL;
  _loadedURL = [nextURL copy];

  if ([url isFileURL]) {
    NSURL *readAccessURL = url.URLByDeletingLastPathComponent ?: url;
    [_webView loadFileURL:url allowingReadAccessToURL:readAccessURL];
    return;
  }

  [_webView loadRequest:[NSURLRequest requestWithURL:url]];
}

- (NSDictionary<NSString *, id> *)historySnapshot
{
  NSMutableArray<NSString *> *entries = [NSMutableArray array];
  for (WKBackForwardListItem *item in _webView.backForwardList.backList) {
    [entries addObject:ServoKitURLString(item.URL)];
  }

  NSInteger current = entries.count;
  WKBackForwardListItem *currentItem = _webView.backForwardList.currentItem;
  NSString *currentURL = ServoKitURLString(currentItem.URL ?: _webView.URL);
  if (currentURL.length > 0) {
    [entries addObject:currentURL];
  }

  for (WKBackForwardListItem *item in _webView.backForwardList.forwardList) {
    [entries addObject:ServoKitURLString(item.URL)];
  }

  return @{
    @"entries" : entries,
    @"current" : @(current),
    @"canGoBack" : @(_webView.canGoBack),
    @"canGoForward" : @(_webView.canGoForward),
  };
}

- (void)observeLoadStatus:(NSString *)status url:(NSString *)url
{
  NSMutableDictionary<NSString *, id> *observation = [[self historySnapshot] mutableCopy];
  observation[@"version"] = @1;
  observation[@"observation"] = @"loadStatusChanged";
  observation[@"status"] = status;
  observation[@"url"] = url ?: @"";
  [self dispatchControllerEnvelope:observation];
}

- (void)observeHistoryChanged
{
  NSMutableDictionary<NSString *, id> *observation = [[self historySnapshot] mutableCopy];
  observation[@"version"] = @1;
  observation[@"observation"] = @"historyChanged";
  [self dispatchControllerEnvelope:observation];
}

- (NSString *)currentNavigationURLString:(WKNavigation *)navigation
{
  NSString *currentURL = ServoKitURLString(_webView.URL);
  if (currentURL.length > 0) {
    return currentURL;
  }
  return _loadedURL ?: _url ?: @"";
}

- (void)observeValueForKeyPath:(NSString *)keyPath ofObject:(id)object change:(NSDictionary<NSKeyValueChangeKey,id> *)change context:(void *)context
{
  if (object != _webView) {
    return;
  }
  if ([keyPath isEqualToString:@"title"]) {
    [self dispatchControllerEnvelope:@{
      @"version" : @1,
      @"observation" : @"pageTitleChanged",
      @"title" : _webView.title ?: NSNull.null,
    }];
    return;
  }

  [super observeValueForKeyPath:keyPath ofObject:object change:change context:context];
}

- (void)webView:(WKWebView *)webView runJavaScriptAlertPanelWithMessage:(NSString *)message initiatedByFrame:(WKFrameInfo *)frame completionHandler:(void (^)(void))completionHandler
{
  if (webView != _webView) {
    completionHandler();
    return;
  }
  [self requestJavaScriptDialogWithKind:ServoKitJavaScriptDialogKindAlert
                                message:message
                           defaultValue:nil
                      completionHandler:^(__unused BOOL confirmed, __unused NSString *promptValue) {
                        completionHandler();
                      }];
}

- (void)webView:(WKWebView *)webView runJavaScriptConfirmPanelWithMessage:(NSString *)message initiatedByFrame:(WKFrameInfo *)frame completionHandler:(void (^)(BOOL result))completionHandler
{
  if (webView != _webView) {
    completionHandler(NO);
    return;
  }
  [self requestJavaScriptDialogWithKind:ServoKitJavaScriptDialogKindConfirm
                                message:message
                           defaultValue:nil
                      completionHandler:^(BOOL confirmed, __unused NSString *promptValue) {
                        completionHandler(confirmed);
                      }];
}

- (void)webView:(WKWebView *)webView runJavaScriptTextInputPanelWithPrompt:(NSString *)prompt defaultText:(NSString *)defaultText initiatedByFrame:(WKFrameInfo *)frame completionHandler:(void (^)(NSString *_Nullable result))completionHandler
{
  if (webView != _webView) {
    completionHandler(nil);
    return;
  }
  [self requestJavaScriptDialogWithKind:ServoKitJavaScriptDialogKindPrompt
                                message:prompt
                           defaultValue:defaultText
                      completionHandler:^(BOOL confirmed, NSString *_Nullable promptValue) {
                        completionHandler(confirmed ? (promptValue ?: defaultText ?: @"") : nil);
                      }];
}

- (void)webView:(WKWebView *)webView decidePolicyForNavigationAction:(WKNavigationAction *)navigationAction decisionHandler:(void (^)(WKNavigationActionPolicy))decisionHandler
{
  if (webView != _webView) {
    decisionHandler(WKNavigationActionPolicyAllow);
    return;
  }
  NSString *url = ServoKitURLString(navigationAction.request.URL);
  [self dispatchControllerEnvelope:@{
    @"version" : @1,
    @"observation" : @"navigationRequest",
    @"url" : url,
  }
              navigationCompletion:decisionHandler
                  dialogCompletion:nil];
}

- (void)webView:(WKWebView *)webView didStartProvisionalNavigation:(WKNavigation *)navigation
{
  if (webView != _webView) {
    return;
  }
  NSString *url = [self currentNavigationURLString:navigation];
  [self observeLoadStatus:ServoKitLoadStatusStarted url:url];
}

- (void)webView:(WKWebView *)webView didCommitNavigation:(WKNavigation *)navigation
{
  if (webView != _webView) {
    return;
  }
  NSString *url = [self currentNavigationURLString:navigation];
  [self observeLoadStatus:ServoKitLoadStatusHeadParsed url:url];
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation
{
  if (webView != _webView) {
    return;
  }
  NSString *url = [self currentNavigationURLString:navigation];
  [self observeLoadStatus:ServoKitLoadStatusComplete url:url];
  [self dispatchControllerEnvelope:@{
    @"version" : @1,
    @"observation" : @"pageTitleChanged",
    @"title" : webView.title ?: NSNull.null,
  }];
}

- (void)webView:(WKWebView *)webView didFailNavigation:(WKNavigation *)navigation withError:(NSError *)error
{
  if (webView != _webView) {
    return;
  }
  [self observeHistoryChanged];
}

- (void)webView:(WKWebView *)webView didFailProvisionalNavigation:(WKNavigation *)navigation withError:(NSError *)error
{
  if (webView != _webView) {
    return;
  }
  [self observeHistoryChanged];
}

@end

#ifdef RCT_NEW_ARCH_ENABLED

using namespace facebook::react;

@interface ServoView : RCTViewComponentView <RCTServoViewViewProtocol>
@end

@implementation ServoView {
  ServoKitWKWebView *_wkWebView;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<ServoViewComponentDescriptor>();
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if ((self = [super initWithFrame:frame])) {
    _wkWebView = [[ServoKitWKWebView alloc] initWithFrame:self.bounds];
    _wkWebView.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    [self configureEventCallbacks];
    self.contentView = _wkWebView;
  }
  return self;
}

- (void)configureEventCallbacks
{
  __weak __typeof(self) weakSelf = self;
  _wkWebView.onUrlChanged = ^(NSString *url) {
    [weakSelf emitUrlChanged:url];
  };
  _wkWebView.onPageTitleChanged = ^(NSString *title) {
    [weakSelf emitPageTitleChanged:title];
  };
  _wkWebView.onLoadStatusChanged = ^(NSString *url, NSString *status) {
    [weakSelf emitLoadStatusChanged:status url:url];
  };
  _wkWebView.onHistoryChanged = ^(NSArray<NSString *> *entries, NSInteger current, BOOL canGoBack, BOOL canGoForward) {
    [weakSelf emitHistoryChanged:entries current:current canGoBack:canGoBack canGoForward:canGoForward];
  };
  _wkWebView.onFocusChanged = ^(BOOL isFocused) {
    [weakSelf emitFocusChanged:isFocused];
  };
  _wkWebView.onJavaScriptEvaluationResult = ^(NSString *evaluationId, BOOL ok, NSString *valueJson, NSString *errorType) {
    [weakSelf emitJavaScriptEvaluationResult:evaluationId ok:ok valueJson:valueJson errorType:errorType];
  };
  _wkWebView.onJavaScriptDialogRequested = ^BOOL(NSString *dialogId, NSString *kind, NSString *message, NSString *_Nullable defaultValue) {
    return [weakSelf emitJavaScriptDialogRequested:dialogId kind:kind message:message defaultValue:defaultValue];
  };
  _wkWebView.onJavaScriptDialogDismissed = ^(NSString *dialogId) {
    [weakSelf emitJavaScriptDialogDismissed:dialogId];
  };
  _wkWebView.onShouldStartLoadWithRequestRequested = ^BOOL(NSString *navigationId, NSString *url) {
    return [weakSelf emitShouldStartLoadWithRequestRequested:navigationId url:url];
  };
}

- (void)updateProps:(Props::Shared const &)props oldProps:(Props::Shared const &)oldProps
{
  const auto &newViewProps = *std::static_pointer_cast<const ServoViewProps>(props);
  _wkWebView.useReactNativeJavaScriptDialogs = newViewProps.useReactNativeJavaScriptDialogs;
  _wkWebView.useReactNativeOnShouldStartLoadWithRequest = newViewProps.useReactNativeOnShouldStartLoadWithRequest;
  _wkWebView.url = [NSString stringWithUTF8String:newViewProps.url.c_str()];

  [super updateProps:props oldProps:oldProps];
}

- (void)handleCommand:(const NSString *)commandName args:(const NSArray *)args
{
  RCTServoViewHandleCommand(self, commandName, args);
}

- (void)sendControllerCommand:(NSString *)commandJson
{
  [_wkWebView sendCommandJson:commandJson];
}

- (void)prepareForRecycle
{
  [super prepareForRecycle];
  [_wkWebView prepareForReuse];
}

- (void)emitUrlChanged:(NSString *)url
{
  if (_eventEmitter == nullptr) {
    return;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnUrlChanged event = {.url = ServoKitStdString(url)};
  eventEmitter->onUrlChanged(event);
}

- (void)emitPageTitleChanged:(NSString *)title
{
  if (_eventEmitter == nullptr) {
    return;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnPageTitleChanged event = {
      .title = title != nil ? ServoKitStdString(title) : std::string(""),
  };
  eventEmitter->onPageTitleChanged(event);
}

- (void)emitLoadStatusChanged:(NSString *)status url:(NSString *)url
{
  if (_eventEmitter == nullptr) {
    return;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnLoadStatusChanged event = {
      .status = ServoKitStdString(status),
  };
  eventEmitter->onLoadStatusChanged(event);
}

- (void)emitHistoryChanged:(NSArray<NSString *> *)entries current:(NSInteger)current canGoBack:(BOOL)canGoBack canGoForward:(BOOL)canGoForward
{
  if (_eventEmitter == nullptr) {
    return;
  }

  std::vector<std::string> nativeEntries;
  nativeEntries.reserve(entries.count);
  for (NSString *entry in entries) {
    nativeEntries.push_back(ServoKitStdString(entry));
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnHistoryChanged event = {
      .entries = nativeEntries,
      .current = static_cast<int>(current),
      .canGoBack = static_cast<bool>(canGoBack),
      .canGoForward = static_cast<bool>(canGoForward),
  };
  eventEmitter->onHistoryChanged(event);
}

- (void)emitFocusChanged:(BOOL)isFocused
{
  if (_eventEmitter == nullptr) {
    return;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnFocusChanged event = {.isFocused = static_cast<bool>(isFocused)};
  eventEmitter->onFocusChanged(event);
}

- (void)emitJavaScriptEvaluationResult:(NSString *)evaluationId ok:(BOOL)ok valueJson:(NSString *)valueJson errorType:(NSString *)errorType
{
  if (_eventEmitter == nullptr) {
    return;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnJavaScriptEvaluationResult event = {
      .evaluationId = ServoKitStdString(evaluationId),
      .ok = static_cast<bool>(ok),
      .valueJson = valueJson != nil ? ServoKitStdString(valueJson) : std::string(""),
      .errorType = errorType != nil ? ServoKitStdString(errorType) : std::string(""),
  };
  eventEmitter->onJavaScriptEvaluationResult(event);
}

- (BOOL)emitJavaScriptDialogRequested:(NSString *)dialogId kind:(NSString *)kind message:(NSString *)message defaultValue:(NSString *_Nullable)defaultValue
{
  if (_eventEmitter == nullptr) {
    return NO;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnJavaScriptDialogRequested event = {
      .dialogId = ServoKitStdString(dialogId),
      .kind = ServoKitStdString(kind),
      .message = ServoKitStdString(message),
      .defaultValue = defaultValue != nil ? ServoKitStdString(defaultValue) : std::string(""),
  };
  eventEmitter->onJavaScriptDialogRequested(event);
  return YES;
}

- (void)emitJavaScriptDialogDismissed:(NSString *)dialogId
{
  if (_eventEmitter == nullptr) {
    return;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnJavaScriptDialogDismissed event = {
      .dialogId = ServoKitStdString(dialogId),
  };
  eventEmitter->onJavaScriptDialogDismissed(event);
}

- (BOOL)emitShouldStartLoadWithRequestRequested:(NSString *)navigationId url:(NSString *)url
{
  if (_eventEmitter == nullptr) {
    return NO;
  }

  auto eventEmitter = std::static_pointer_cast<const ServoViewEventEmitter>(_eventEmitter);
  ServoViewEventEmitter::OnShouldStartLoadWithRequestRequested event = {
      .navigationId = ServoKitStdString(navigationId),
      .url = ServoKitStdString(url),
  };
  eventEmitter->onShouldStartLoadWithRequestRequested(event);
  return YES;
}

@end

Class<RCTComponentViewProtocol> ServoViewCls(void)
{
  return ServoView.class;
}

#else

@interface ServoViewManager : RCTViewManager
@end

@implementation ServoViewManager

RCT_EXPORT_MODULE(ServoView)
RCT_EXPORT_VIEW_PROPERTY(url, NSString)
RCT_EXPORT_VIEW_PROPERTY(useReactNativeJavaScriptDialogs, BOOL)
RCT_EXPORT_VIEW_PROPERTY(useReactNativeOnShouldStartLoadWithRequest, BOOL)

- (UIView *)view
{
  return [[ServoKitWKWebView alloc] initWithFrame:CGRectZero];
}

@end

#endif
