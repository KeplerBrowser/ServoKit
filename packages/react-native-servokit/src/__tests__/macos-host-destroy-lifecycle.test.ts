import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

import { expect, test } from 'bun:test';

const macOSTest = process.platform === 'darwin' ? test : test.skip;

macOSTest('macOS destroy transfers borrowed lifetimes and retries on the main queue', () => {
  const temporaryDirectory = mkdtempSync(
    join(tmpdir(), 'servokit-destroy-lifecycle-')
  );
  const includeDirectory = join(temporaryDirectory, 'ServoKit');
  const harness = join(temporaryDirectory, 'destroy-lifecycle.mm');
  const executable = join(temporaryDirectory, 'destroy-lifecycle');
  const helper = fileURLToPath(
    new URL('../../macos/ServoHostDestroy.h', import.meta.url)
  );
  const abiHeader = fileURLToPath(
    new URL(
      '../../../../crates/servokit-host-desktop/include/servokit_desktop_private.h',
      import.meta.url
    )
  );

  try {
    mkdirSync(includeDirectory);
    copyFileSync(
      abiHeader,
      join(includeDirectory, 'servokit_desktop_private.h')
    );
    writeFileSync(
      harness,
      String.raw`
#include <atomic>
#include <cstdio>
#include <initializer_list>

#import <Foundation/Foundation.h>

#import "ServoHostDestroy.h"

#define CHECK(condition) \
  do { \
    if (!(condition)) { \
      std::fprintf(stderr, "check failed at line %d: %s\n", __LINE__, #condition); \
      std::fflush(stderr); \
      std::_Exit(__LINE__); \
    } \
  } while (false)

static std::atomic<int> callCount;
static ServokitDesktopPrivateStatus statuses[8];
static std::atomic<bool> calledOnMain[8];
static char mainQueueKey;

static void Reset(std::initializer_list<ServokitDesktopPrivateStatus> values)
{
  callCount = 0;
  int index = 0;
  for (ServokitDesktopPrivateStatus value : values) {
    statuses[index] = value;
    calledOnMain[index] = false;
    index += 1;
  }
}

static ServokitDesktopPrivateHost *FakeHost()
{
  return reinterpret_cast<ServokitDesktopPrivateHost *>(0x1234);
}

static ServokitDesktopPrivateStatus FakeDestroy(
    ServokitDesktopPrivateHost *host, uint64_t token)
{
  CHECK(host == FakeHost());
  CHECK(token == 42);
  int index = callCount.fetch_add(1);
  calledOnMain[index] =
      dispatch_get_specific(&mainQueueKey) == &mainQueueKey;
  return statuses[index];
}

@interface Lifetime : NSObject
@end

@implementation Lifetime
@end

static void AssertTerminalStatus(ServokitDesktopPrivateStatus status)
{
  @autoreleasepool {
    Reset({status});
    ServokitDesktopPrivateHost *host = FakeHost();
    uint64_t token = 42;
    Lifetime *callbackLifetime = [Lifetime new];
    Lifetime *nativeLifetime = [Lifetime new];
    __weak Lifetime *weakCallbackLifetime = callbackLifetime;
    __weak Lifetime *weakNativeLifetime = nativeLifetime;

    CHECK(ServoKitDestroyHost(
        &host,
        &token,
        callbackLifetime,
        nativeLifetime,
        FakeDestroy));
    callbackLifetime = nil;
    nativeLifetime = nil;

    CHECK(host == nullptr);
    CHECK(token == 0);
    CHECK(weakCallbackLifetime == nil);
    CHECK(weakNativeLifetime == nil);
    CHECK(callCount == 1);
    CHECK(!ServoKitDestroyHost(
        &host,
        &token,
        nil,
        nil,
        FakeDestroy));
    CHECK(callCount == 1);
  }
}

static void RunWrongThreadScenario()
{
  dispatch_async(dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
    @autoreleasepool {
      Reset({
          SERVOKIT_DESKTOP_PRIVATE_WRONG_THREAD,
          SERVOKIT_DESKTOP_PRIVATE_OK,
      });
      ServokitDesktopPrivateHost *host = FakeHost();
      uint64_t token = 42;
      Lifetime *callbackLifetime = [Lifetime new];
      Lifetime *nativeLifetime = [Lifetime new];
      __weak Lifetime *weakCallbackLifetime = callbackLifetime;
      __weak Lifetime *weakNativeLifetime = nativeLifetime;

      CHECK(ServoKitDestroyHost(
          &host,
          &token,
          callbackLifetime,
          nativeLifetime,
          FakeDestroy));
      CHECK(host == nullptr);
      CHECK(token == 0);
      CHECK(!calledOnMain[0]);
      CHECK(!ServoKitDestroyHost(
          &host,
          &token,
          nil,
          nil,
          FakeDestroy));
      CHECK(callCount == 1);

      callbackLifetime = nil;
      nativeLifetime = nil;
      CHECK(weakCallbackLifetime != nil);
      CHECK(weakNativeLifetime != nil);

      dispatch_async(dispatch_get_main_queue(), ^{
        CHECK(callCount == 2);
        CHECK(calledOnMain[1]);
        CHECK(weakCallbackLifetime == nil);
        CHECK(weakNativeLifetime == nil);
        exit(0);
      });
    }
  });
}

int main()
{
  @autoreleasepool {
    dispatch_queue_set_specific(
        dispatch_get_main_queue(),
        &mainQueueKey,
        &mainQueueKey,
        nullptr);
    dispatch_async(dispatch_get_main_queue(), ^{
      @autoreleasepool {
        Reset({
            SERVOKIT_DESKTOP_PRIVATE_BUSY,
            SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN,
        });
        ServokitDesktopPrivateHost *host = FakeHost();
        uint64_t token = 42;
        Lifetime *callbackLifetime = [Lifetime new];
        Lifetime *nativeLifetime = [Lifetime new];
        __weak Lifetime *weakCallbackLifetime = callbackLifetime;
        __weak Lifetime *weakNativeLifetime = nativeLifetime;

        CHECK(ServoKitDestroyHost(
            &host,
            &token,
            callbackLifetime,
            nativeLifetime,
            FakeDestroy));
        CHECK(host == nullptr);
        CHECK(token == 0);
        CHECK(calledOnMain[0]);
        CHECK(!ServoKitDestroyHost(
            &host,
            &token,
            nil,
            nil,
            FakeDestroy));
        CHECK(callCount == 1);

        callbackLifetime = nil;
        nativeLifetime = nil;
        CHECK(weakCallbackLifetime != nil);
        CHECK(weakNativeLifetime != nil);

        dispatch_async(dispatch_get_main_queue(), ^{
          CHECK(callCount == 2);
          CHECK(calledOnMain[1]);
          CHECK(weakCallbackLifetime == nil);
          CHECK(weakNativeLifetime == nil);

          AssertTerminalStatus(SERVOKIT_DESKTOP_PRIVATE_OK);
          AssertTerminalStatus(SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR);
          AssertTerminalStatus(SERVOKIT_DESKTOP_PRIVATE_PANIC);
          AssertTerminalStatus(SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN);
          AssertTerminalStatus(SERVOKIT_DESKTOP_PRIVATE_NULL_POINTER);
          AssertTerminalStatus(SERVOKIT_DESKTOP_PRIVATE_INVALID_ARGUMENT);
          RunWrongThreadScenario();
        });
      }
    });

    dispatch_after(
        dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC),
        dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0),
        ^{
          abort();
        });
    dispatch_main();
  }
}
`
    );

    const compile = spawnSync(
      'xcrun',
      [
        'clang++',
        '-std=c++17',
        '-fobjc-arc',
        '-fblocks',
        '-Wall',
        '-Wextra',
        '-Werror',
        '-framework',
        'Foundation',
        '-I',
        temporaryDirectory,
        '-I',
        dirname(helper),
        harness,
        '-o',
        executable,
      ],
      { encoding: 'utf8' }
    );
    expect(compile.status, compile.stderr).toBe(0);

    const run = spawnSync(executable, {
      encoding: 'utf8',
      timeout: 10_000,
    });
    expect(
      run.status,
      `signal=${run.signal} error=${run.error}\n${run.stdout}\n${run.stderr}`
    ).toBe(0);
  } finally {
    rmSync(temporaryDirectory, { recursive: true, force: true });
  }
});
