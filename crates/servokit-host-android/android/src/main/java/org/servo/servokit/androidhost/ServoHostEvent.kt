package org.servo.servokit.androidhost

/**
 * Proof-only Kotlin bridge types for the reusable Android host module.
 *
 * These types are intentionally narrow and may change as the Android host
 * extraction continues; they are not a stable public Kotlin SDK.
 */
data class SelectElementOption(
  val id: Int,
  val label: String,
  val isDisabled: Boolean
)

sealed interface SelectElementOptionOrOptgroup {
  data class Option(val option: SelectElementOption) : SelectElementOptionOrOptgroup

  data class Optgroup(
    val label: String,
    val options: List<SelectElementOption>
  ) : SelectElementOptionOrOptgroup
}

sealed interface ContextMenuItem {
  data class Item(
    val label: String,
    val action: String,
    val enabled: Boolean
  ) : ContextMenuItem

  data object Separator : ContextMenuItem
}

data class ContextMenuElementInformation(
  val isLink: Boolean,
  val isImage: Boolean,
  val isEditableText: Boolean,
  val hasSelection: Boolean,
  val linkUrl: String?,
  val imageUrl: String?,
  val contextType: String
)

sealed interface ServoHostEvent {
  data class NavigationRequested(
    val navigationId: String,
    val url: String
  ) : ServoHostEvent

  data class PopupRequested(
    val parentWebViewId: String,
    val parentUrl: String?,
    val targetUrl: String?,
    val windowFeatures: String?,
    val policy: String
  ) : ServoHostEvent

  data class PopupCreated(
    val parentWebViewId: String,
    val childWebViewId: String,
    val parentUrl: String?,
    val targetUrl: String?,
    val windowFeatures: String?,
    val policy: String
  ) : ServoHostEvent

  data class UrlChanged(val url: String) : ServoHostEvent

  data class PageTitleChanged(val title: String?) : ServoHostEvent

  data class StatusTextChanged(val status: String?) : ServoHostEvent

  data class LoadStatusChanged(val status: String) : ServoHostEvent

  data class HistoryChanged(
    val entries: List<String>,
    val current: Int,
    val canGoBack: Boolean,
    val canGoForward: Boolean
  ) : ServoHostEvent

  data object Closed : ServoHostEvent

  data class Crashed(
    val url: String?,
    val reason: String,
    val backtrace: String?
  ) : ServoHostEvent

  data class Error(
    val url: String?,
    val code: Int,
    val message: String
  ) : ServoHostEvent

  data class JavaScriptEvaluationResult(
    val evaluationId: String,
    val ok: Boolean,
    val valueJson: String?,
    val errorType: String?
  ) : ServoHostEvent

  data class SimpleDialogRequested(
    val dialogId: String,
    val kind: String,
    val message: String,
    val defaultValue: String?
  ) : ServoHostEvent

  data class SimpleDialogDismissed(val dialogId: String) : ServoHostEvent

  data class InputMethodRequested(
    val inputMethodId: String,
    val type: String,
    val text: String,
    val insertionPoint: Int?,
    val multiline: Boolean,
    val allowVirtualKeyboard: Boolean
  ) : ServoHostEvent

  data class InputMethodDismissed(val inputMethodId: String) : ServoHostEvent

  data class SelectElementRequested(
    val selectElementId: String,
    val options: List<SelectElementOptionOrOptgroup>,
    val selectedOptions: List<Int>,
    val allowSelectMultiple: Boolean
  ) : ServoHostEvent

  data class SelectElementDismissed(val selectElementId: String) : ServoHostEvent

  data class ContextMenuRequested(
    val contextMenuId: String,
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int,
    val elementInfo: ContextMenuElementInformation,
    val items: List<ContextMenuItem>
  ) : ServoHostEvent

  data class ContextMenuDismissed(val contextMenuId: String) : ServoHostEvent

  data class FilePickerRequested(
    val filePickerId: String,
    val currentPaths: List<String>,
    val filterPatterns: List<String>,
    val allowSelectMultiple: Boolean
  ) : ServoHostEvent

  data class FilePickerDismissed(val filePickerId: String) : ServoHostEvent

  data class PermissionRequested(
    val permission: String,
    val origin: String
  ) : ServoHostEvent

  data class FocusChanged(val isFocused: Boolean) : ServoHostEvent

  data class CursorChanged(val cursor: String) : ServoHostEvent

  data class FullscreenChanged(val isFullscreen: Boolean) : ServoHostEvent

  data class SurfaceAttached(val width: Int, val height: Int) : ServoHostEvent

  data class SurfaceResized(val width: Int, val height: Int) : ServoHostEvent

  data object SurfaceDetached : ServoHostEvent
}
