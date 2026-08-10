package org.servo.servokit.androidhost

import android.content.Context
import android.view.Surface

private const val STATUS_NULL_POINTER = 2
private const val DISPOSED_MESSAGE = "binding has been disposed"

/**
 * Proof-only Android host/control coordinator shared by repo-local consumers.
 *
 * React Native Android and Kotlin Android examples use this class to send the
 * same browser, surface, and embedder-control commands through
 * `crates/servokit-host-android/android`. It is intentionally not a stable public
 * Kotlin SDK or AAR API.
 */
class ServoViewBinding(
  private val host: ServoHost
) {
  constructor(applicationContext: Context) : this(JniServoHost(applicationContext))

  private var disposed = false

  @Synchronized
  fun controllerHandle(): String? {
    if (disposed) {
      return null
    }

    return host.controllerHandle()
  }

  @Synchronized
  fun loadUrl(input: String): List<ServoHostEvent> {
    if (disposed) {
      return listOf(ServoHostEvent.Error(input, STATUS_NULL_POINTER, DISPOSED_MESSAGE))
    }

    host.loadUrl(input)
    return host.drainEvents()
  }

  @Synchronized
  fun reload(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.reload()
    return host.drainEvents()
  }

  @Synchronized
  fun goBack(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.goBack()
    return host.drainEvents()
  }

  @Synchronized
  fun goForward(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.goForward()
    return host.drainEvents()
  }

  @Synchronized
  fun focus(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.focus()
    return host.drainEvents()
  }

  @Synchronized
  fun blur(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.blur()
    return host.drainEvents()
  }

  @Synchronized
  fun sendControllerCommand(commandJson: String): Int {
    if (disposed) {
      return STATUS_NULL_POINTER
    }

    val controllerHandle = host.controllerHandle() ?: return STATUS_NULL_POINTER
    return host.sendControllerCommand(controllerHandle, commandJson)
  }

  @Synchronized
  fun resolveSimpleDialog(
    dialogId: String,
    action: Int,
    promptValue: String?
  ): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolveSimpleDialog(dialogId, action, promptValue)
    return host.drainEvents()
  }

  @Synchronized
  fun resolveSelectElement(selectElementId: String, selectedOptions: IntArray): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolveSelectElement(selectElementId, selectedOptions)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun resolveContextMenu(contextMenuId: String, action: String): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolveContextMenu(contextMenuId, action)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun resolveContextMenu(
    controllerHandle: String,
    contextMenuId: String,
    action: String
  ): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolveContextMenu(controllerHandle, contextMenuId, action)
    return emptyList()
  }

  @Synchronized
  fun dismissContextMenu(contextMenuId: String): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dismissContextMenu(contextMenuId)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun dismissContextMenu(controllerHandle: String, contextMenuId: String): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dismissContextMenu(controllerHandle, contextMenuId)
    return emptyList()
  }

  @Synchronized
  fun resolveFilePicker(filePickerId: String, selectedPaths: Array<String>): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolveFilePicker(filePickerId, selectedPaths)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun dismissFilePicker(filePickerId: String): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dismissFilePicker(filePickerId)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun hasPendingPermissionRequest(): Boolean {
    if (disposed) {
      return false
    }

    return host.hasPendingPermissionRequest()
  }

  @Synchronized
  fun getPendingPermissionRequest(): ServoHostEvent.PermissionRequested? {
    if (disposed) {
      return null
    }

    return host.getPendingPermissionRequest()
  }

  @Synchronized
  fun resolvePermission(allow: Boolean): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolvePermission(allow)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun resolveNavigationRequest(navigationId: String, allow: Boolean): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.resolveNavigationRequest(navigationId, allow)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun dispatchImeComposition(state: Int, text: String): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dispatchImeComposition(state, text)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun dismissInputMethod(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dismissInputMethod()
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun dispatchKeyboardKey(key: Int): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dispatchKeyboardKey(key)
    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun attachSurface(
    surface: Surface,
    width: Int,
    height: Int,
    density: Float
  ): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    validateSurfaceMetrics(width, height, density)
    host.attachSurface(surface, width, height, density)
    return host.drainEvents()
  }

  @Synchronized
  fun resizeSurface(width: Int, height: Int, density: Float): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    validateSurfaceMetrics(width, height, density)
    host.resizeSurface(width, height, density)
    return host.drainEvents()
  }

  @Synchronized
  fun detachSurface(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.detachSurface()
    return host.drainEvents()
  }

  @Synchronized
  fun performUpdates(): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.performUpdates()
    return host.drainEvents()
  }

  @Synchronized
  fun dispatchTouchEvent(action: Int, pointerId: Int, x: Float, y: Float): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.dispatchTouchEvent(action, pointerId, x, y)
    return host.drainEvents()
  }

  @Synchronized
  fun triggerContextMenu(x: Float, y: Float): List<ServoHostEvent> {
    if (disposed) {
      return emptyList()
    }

    host.triggerContextMenu(x, y)
    return host.drainEvents()
  }

  @Synchronized
  fun dispose() {
    if (disposed) {
      return
    }

    disposed = true
    host.destroy()
  }

  private fun validateSurfaceMetrics(width: Int, height: Int, density: Float) {
    require(width >= 0) { "surface width must be non-negative" }
    require(height >= 0) { "surface height must be non-negative" }
    require(density.isFinite()) { "surface density must be finite" }
    require(density > 0f) { "surface density must be positive" }
  }
}
