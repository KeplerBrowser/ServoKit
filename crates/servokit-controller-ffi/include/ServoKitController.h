#ifndef SERVOKIT_CONTROLLER_H
#define SERVOKIT_CONTROLLER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
  SERVOKIT_CONTROLLER_OK = 0,
  SERVOKIT_CONTROLLER_INVALID_ARGUMENT = 1,
  SERVOKIT_CONTROLLER_STALE_HANDLE = 2,
  SERVOKIT_CONTROLLER_BUSY = 3,
  SERVOKIT_CONTROLLER_INTERNAL_ERROR = 4,
  SERVOKIT_CONTROLLER_PANIC = 5,
};

typedef struct {
  uint32_t status;
  uint64_t handle;
  const uint8_t *bytes;
  size_t len;
} ServoKitControllerResult;

// Calls for one handle are synchronous and must be serialized. The iOS adapter calls these on
// its main thread; the ABI itself does not claim or enforce a UI thread.
ServoKitControllerResult servokit_controller_create(void);
ServoKitControllerResult servokit_controller_dispatch(
    uint64_t handle, const uint8_t *bytes, size_t len);
ServoKitControllerResult servokit_controller_destroy(uint64_t handle);

// Result bytes are exact-length, read-only, and not NUL-terminated. Copy them before freeing.
// Call exactly once for every returned result. Null/zero empty results are accepted, and free is
// thread-agnostic. Mutation, double-free, and access after free are invalid caller behavior.
void servokit_controller_result_free(ServoKitControllerResult result);

#ifdef __cplusplus
}
#endif

#endif
