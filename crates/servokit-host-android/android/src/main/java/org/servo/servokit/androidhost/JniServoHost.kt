package org.servo.servokit.androidhost

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.view.Surface

/**
 * Minimal JNI owner for repo-local Android hosts.
 *
 * This module packages the `servokit_host_android` native library and exposes a
 * proof-only Kotlin wrapper for the current React Native adapter and native
 * Android proof app. It is not a stable public Kotlin SDK.
 */
class JniServoHost(context: Context) : ServoHost {
  private val clipboardContext = context.applicationContext.also(::installApplicationContext)
  private var handle = nativeCreateHost()

  override fun controllerHandle(): String? {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return null
    }

    return nativeControllerHandle(activeHandle).takeIf { it.isNotEmpty() }
  }

  override fun loadUrl(input: String): Int {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return STATUS_NULL_POINTER
    }

    return nativeLoadUrl(activeHandle, input)
  }

  override fun reload() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeReload(activeHandle)
  }

  override fun goBack() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeGoBack(activeHandle)
  }

  override fun goForward() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeGoForward(activeHandle)
  }

  override fun focus() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeFocus(activeHandle)
  }

  override fun blur() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeBlur(activeHandle)
  }

  override fun resolveSimpleDialog(dialogId: String, action: Int, promptValue: String?) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResolveSimpleDialog(activeHandle, dialogId, action, promptValue)
  }

  override fun resolveSelectElement(selectElementId: String, selectedOptions: IntArray) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResolveSelectElement(activeHandle, selectElementId, selectedOptions)
  }

  override fun resolveContextMenu(contextMenuId: String, action: String) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResolveContextMenu(activeHandle, contextMenuId, action)
  }

  override fun resolveContextMenu(
    controllerHandle: String,
    contextMenuId: String,
    action: String
  ): Int = nativeResolveContextMenuWithController(controllerHandle, contextMenuId, action)

  override fun dismissContextMenu(contextMenuId: String) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDismissContextMenu(activeHandle, contextMenuId)
  }

  override fun dismissContextMenu(
    controllerHandle: String,
    contextMenuId: String
  ): Int = nativeDismissContextMenuWithController(controllerHandle, contextMenuId)

  override fun sendControllerCommand(controllerHandle: String, commandJson: String): Int =
    nativeSendControllerCommandWithController(controllerHandle, commandJson)

  override fun resolveFilePicker(filePickerId: String, selectedPaths: Array<String>) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResolveFilePicker(activeHandle, filePickerId, selectedPaths)
  }

  override fun dismissFilePicker(filePickerId: String) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDismissFilePicker(activeHandle, filePickerId)
  }

  override fun hasPendingPermissionRequest(): Boolean {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return false
    }

    return nativeHasPendingPermissionRequest(activeHandle)
  }

  override fun getPendingPermissionRequest(): ServoHostEvent.PermissionRequested? {
    val activeHandle = handle
    if (activeHandle == 0L || !nativeHasPendingPermissionRequest(activeHandle)) {
      return null
    }

    return ServoHostEvent.PermissionRequested(
      permission = nativePendingPermissionRequestPermission(activeHandle),
      origin = nativePendingPermissionRequestOrigin(activeHandle)
    )
  }

  override fun resolvePermission(allow: Boolean) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResolvePermission(activeHandle, allow)
  }

  override fun resolveNavigationRequest(navigationId: String, allow: Boolean) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResolveNavigationRequest(activeHandle, navigationId, allow)
  }

  override fun dispatchImeComposition(state: Int, text: String) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDispatchImeComposition(activeHandle, state, text)
  }

  override fun dismissInputMethod() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDismissInputMethod(activeHandle)
  }

  override fun dispatchKeyboardKey(key: Int) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDispatchKeyboardKey(activeHandle, key)
  }

  override fun attachSurface(surface: Surface, width: Int, height: Int, density: Float) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeAttachSurface(activeHandle, surface, width, height, density)
  }

  override fun resizeSurface(width: Int, height: Int, density: Float) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeResizeSurface(activeHandle, width, height, density)
  }

  override fun detachSurface() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDetachSurface(activeHandle)
  }

  override fun performUpdates() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativePerformUpdates(activeHandle)
  }

  override fun dispatchTouchEvent(action: Int, pointerId: Int, x: Float, y: Float) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDispatchTouchEvent(activeHandle, action, pointerId, x, y)
  }

  override fun triggerContextMenu(x: Float, y: Float) {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeTriggerContextMenu(activeHandle, x, y)
  }

  fun drainEventJson(): List<String> {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return emptyList()
    }

    val events = mutableListOf<String>()
    while (true) {
      val eventJson = nativeTakeNextEventBridgeJson(activeHandle) ?: return events
      events += eventJson
    }
  }

  override fun drainEvents(): List<ServoHostEvent> =
    drainEventJson().map(ServoHostEventBridge::decode)

  override fun destroy() {
    val activeHandle = handle
    if (activeHandle == 0L) {
      return
    }

    nativeDestroyHost(activeHandle)
    handle = 0L
  }

  companion object {
    private const val STATUS_NULL_POINTER = 2

    @Volatile
    private var applicationContext: Context? = null

    init {
      System.loadLibrary("servokit_host_android")
    }

    internal fun installApplicationContext(context: Context) {
      applicationContext = context.applicationContext
    }

    @JvmStatic
    fun platformGetClipboardText(): String? {
      val activeContext = applicationContext ?: return null
      val clipboard = activeContext.getSystemService(ClipboardManager::class.java) ?: return null
      val clip = clipboard.primaryClip ?: return ""
      if (clip.itemCount == 0) {
        return ""
      }

      return clip.getItemAt(0).coerceToText(activeContext)?.toString().orEmpty()
    }

    @JvmStatic
    fun platformSetClipboardText(text: String): Boolean {
      val activeContext = applicationContext ?: return false
      val clipboard = activeContext.getSystemService(ClipboardManager::class.java) ?: return false
      clipboard.setPrimaryClip(ClipData.newPlainText("servo", text))
      return true
    }

    @JvmStatic
    fun platformClearClipboardText(): Boolean {
      val activeContext = applicationContext ?: return false
      val clipboard = activeContext.getSystemService(ClipboardManager::class.java) ?: return false
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        clipboard.clearPrimaryClip()
      } else {
        clipboard.setPrimaryClip(ClipData.newPlainText("servo", ""))
      }
      return true
    }

    @JvmStatic
    private external fun nativeCreateHost(): Long

    @JvmStatic
    private external fun nativeLoadUrl(handle: Long, input: String): Int

    @JvmStatic
    private external fun nativeControllerHandle(handle: Long): String

    @JvmStatic
    private external fun nativeReload(handle: Long)

    @JvmStatic
    private external fun nativeGoBack(handle: Long)

    @JvmStatic
    private external fun nativeGoForward(handle: Long)

    @JvmStatic
    private external fun nativeFocus(handle: Long)

    @JvmStatic
    private external fun nativeBlur(handle: Long)

    @JvmStatic
    private external fun nativeResolveSimpleDialog(
      handle: Long,
      dialogId: String,
      action: Int,
      promptValue: String?
    )

    @JvmStatic
    private external fun nativeResolveSelectElement(
      handle: Long,
      selectElementId: String,
      selectedOptions: IntArray
    )

    @JvmStatic
    private external fun nativeResolveContextMenu(handle: Long, contextMenuId: String, action: String)

    @JvmStatic
    private external fun nativeResolveContextMenuWithController(
      controllerHandle: String,
      contextMenuId: String,
      action: String
    ): Int

    @JvmStatic
    private external fun nativeDismissContextMenu(handle: Long, contextMenuId: String)

    @JvmStatic
    private external fun nativeDismissContextMenuWithController(
      controllerHandle: String,
      contextMenuId: String
    ): Int

    @JvmStatic
    private external fun nativeSendControllerCommandWithController(
      controllerHandle: String,
      commandJson: String
    ): Int

    @JvmStatic
    private external fun nativeResolveFilePicker(
      handle: Long,
      filePickerId: String,
      selectedPaths: Array<String>
    )

    @JvmStatic
    private external fun nativeDismissFilePicker(handle: Long, filePickerId: String)

    @JvmStatic
    private external fun nativeHasPendingPermissionRequest(handle: Long): Boolean

    @JvmStatic
    private external fun nativePendingPermissionRequestPermission(handle: Long): String

    @JvmStatic
    private external fun nativePendingPermissionRequestOrigin(handle: Long): String

    @JvmStatic
    private external fun nativeResolvePermission(handle: Long, allow: Boolean): Int

    @JvmStatic
    private external fun nativeResolveNavigationRequest(
      handle: Long,
      navigationId: String,
      allow: Boolean
    ): Int

    @JvmStatic
    private external fun nativeDispatchImeComposition(handle: Long, state: Int, text: String): Int

    @JvmStatic
    private external fun nativeDismissInputMethod(handle: Long): Int

    @JvmStatic
    private external fun nativeDispatchKeyboardKey(handle: Long, key: Int): Int

    @JvmStatic
    private external fun nativeAttachSurface(
      handle: Long,
      surface: Surface,
      width: Int,
      height: Int,
      density: Float
    )

    @JvmStatic
    private external fun nativeResizeSurface(handle: Long, width: Int, height: Int, density: Float)

    @JvmStatic
    private external fun nativeDetachSurface(handle: Long)

    @JvmStatic
    private external fun nativePerformUpdates(handle: Long)

    @JvmStatic
    private external fun nativeDispatchTouchEvent(
      handle: Long,
      action: Int,
      pointerId: Int,
      x: Float,
      y: Float
    )

    @JvmStatic
    private external fun nativeTriggerContextMenu(handle: Long, x: Float, y: Float)

    @JvmStatic
    private external fun nativeTakeNextEventBridgeJson(handle: Long): String?

    @JvmStatic
    private external fun nativeDestroyHost(handle: Long)
  }
}
