package org.servo.servokit.androidhost

import android.view.Surface

const val SIMPLE_DIALOG_ACTION_CONFIRM = 0
const val SIMPLE_DIALOG_ACTION_DISMISS = 1
const val TOUCH_EVENT_DOWN = 0
const val TOUCH_EVENT_MOVE = 1
const val TOUCH_EVENT_UP = 2
const val TOUCH_EVENT_CANCEL = 3
const val IME_COMPOSITION_UPDATE = 0
const val IME_COMPOSITION_END = 1
const val KEYBOARD_KEY_BACKSPACE = 0
const val KEYBOARD_KEY_DELETE = 1
const val KEYBOARD_KEY_ENTER = 2

/**
 * Proof-only low-level Android host interface used by repo-local adapters.
 * This is intentionally not a stable Android SDK contract.
 */
interface ServoHost {
  fun controllerHandle(): String?

  fun loadUrl(input: String): Int

  fun reload()

  fun goBack()

  fun goForward()

  fun focus()

  fun blur()

  fun resolveSimpleDialog(dialogId: String, action: Int, promptValue: String?)

  fun resolveSelectElement(selectElementId: String, selectedOptions: IntArray)

  fun resolveContextMenu(contextMenuId: String, action: String)

  fun resolveContextMenu(controllerHandle: String, contextMenuId: String, action: String): Int

  fun sendControllerCommand(controllerHandle: String, commandJson: String): Int

  fun dismissContextMenu(contextMenuId: String)

  fun dismissContextMenu(controllerHandle: String, contextMenuId: String): Int

  fun resolveFilePicker(filePickerId: String, selectedPaths: Array<String>)

  fun dismissFilePicker(filePickerId: String)

  fun hasPendingPermissionRequest(): Boolean

  fun getPendingPermissionRequest(): ServoHostEvent.PermissionRequested?

  fun resolvePermission(allow: Boolean)

  fun resolveNavigationRequest(navigationId: String, allow: Boolean)

  fun dispatchImeComposition(state: Int, text: String)

  fun dismissInputMethod()

  fun dispatchKeyboardKey(key: Int)

  fun attachSurface(surface: Surface, width: Int, height: Int, density: Float)

  fun resizeSurface(width: Int, height: Int, density: Float)

  fun detachSurface()

  fun performUpdates()

  fun dispatchTouchEvent(action: Int, pointerId: Int, x: Float, y: Float)

  fun triggerContextMenu(x: Float, y: Float)

  fun drainEvents(): List<ServoHostEvent>

  fun destroy()
}
