#pragma once

#import <Foundation/Foundation.h>
#import <ExplorerKit/explorerkit_desktop_private.h>

typedef ExplorerKitDesktopPrivateStatus (*ExplorerKitDestroyHostFunction)(
    ExplorerKitDesktopPrivateHost *host, uint64_t token);

@interface ExplorerKitHostDestroyOwner : NSObject
- (instancetype)initWithHost:(ExplorerKitDesktopPrivateHost *)host
                       token:(uint64_t)token
            callbackLifetime:(id)callbackLifetime
              nativeLifetime:(id)nativeLifetime
                     destroy:(ExplorerKitDestroyHostFunction)destroy;
- (void)destroyHost;
@end

@implementation ExplorerKitHostDestroyOwner {
  ExplorerKitDesktopPrivateHost *_host;
  uint64_t _token;
  id _callbackLifetime;
  id _nativeLifetime;
  ExplorerKitDestroyHostFunction _destroy;
}

- (instancetype)initWithHost:(ExplorerKitDesktopPrivateHost *)host
                       token:(uint64_t)token
            callbackLifetime:(id)callbackLifetime
              nativeLifetime:(id)nativeLifetime
                     destroy:(ExplorerKitDestroyHostFunction)destroy
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

  ExplorerKitDesktopPrivateStatus status = _destroy(_host, _token);
  if (status == EXPLORERKIT_DESKTOP_PRIVATE_WRONG_THREAD ||
      status == EXPLORERKIT_DESKTOP_PRIVATE_BUSY) {
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

static inline BOOL ExplorerKitDestroyHost(
    ExplorerKitDesktopPrivateHost **host,
    uint64_t *token,
    id callbackLifetime,
    id nativeLifetime,
    ExplorerKitDestroyHostFunction destroy)
{
  if (host == nullptr || token == nullptr || *host == nullptr) {
    return NO;
  }

  ExplorerKitHostDestroyOwner *owner =
      [[ExplorerKitHostDestroyOwner alloc] initWithHost:*host
                                              token:*token
                                   callbackLifetime:callbackLifetime
                                     nativeLifetime:nativeLifetime
                                            destroy:destroy];
  *host = nullptr;
  *token = 0;
  [owner destroyHost];
  return YES;
}
