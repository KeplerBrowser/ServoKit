#pragma once

#import <Foundation/Foundation.h>
#import <ServoKit/servokit_desktop_private.h>

typedef ServokitDesktopPrivateStatus (*ServoKitDestroyHostFunction)(
    ServokitDesktopPrivateHost *host, uint64_t token);

@interface ServoKitHostDestroyOwner : NSObject
- (instancetype)initWithHost:(ServokitDesktopPrivateHost *)host
                       token:(uint64_t)token
            callbackLifetime:(id)callbackLifetime
              nativeLifetime:(id)nativeLifetime
                     destroy:(ServoKitDestroyHostFunction)destroy;
- (void)destroyHost;
@end

@implementation ServoKitHostDestroyOwner {
  ServokitDesktopPrivateHost *_host;
  uint64_t _token;
  id _callbackLifetime;
  id _nativeLifetime;
  ServoKitDestroyHostFunction _destroy;
}

- (instancetype)initWithHost:(ServokitDesktopPrivateHost *)host
                       token:(uint64_t)token
            callbackLifetime:(id)callbackLifetime
              nativeLifetime:(id)nativeLifetime
                     destroy:(ServoKitDestroyHostFunction)destroy
{
  if ((self = [super init])) {
    _host = host;
    _token = token;
    _callbackLifetime = callbackLifetime;
    _nativeLifetime = nativeLifetime;
    _destroy = destroy;
  }
  return self;
}

- (void)destroyHost
{
  if (_host == nullptr) {
    return;
  }

  ServokitDesktopPrivateStatus status = _destroy(_host, _token);
  if (status == SERVOKIT_DESKTOP_PRIVATE_WRONG_THREAD ||
      status == SERVOKIT_DESKTOP_PRIVATE_BUSY) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [self destroyHost];
    });
    return;
  }

  // STALE_TOKEN means this unique pair is no longer live; NULL_POINTER and
  // INVALID_ARGUMENT are impossible for the adapter-owned non-null pair.
  _host = nullptr;
  _token = 0;
  _callbackLifetime = nil;
  _nativeLifetime = nil;
}

@end

static inline BOOL ServoKitDestroyHost(
    ServokitDesktopPrivateHost **host,
    uint64_t *token,
    id callbackLifetime,
    id nativeLifetime,
    ServoKitDestroyHostFunction destroy)
{
  if (host == nullptr || token == nullptr || *host == nullptr) {
    return NO;
  }

  ServoKitHostDestroyOwner *owner =
      [[ServoKitHostDestroyOwner alloc] initWithHost:*host
                                              token:*token
                                   callbackLifetime:callbackLifetime
                                     nativeLifetime:nativeLifetime
                                            destroy:destroy];
  *host = nullptr;
  *token = 0;
  [owner destroyHost];
  return YES;
}
