package org.servo.servokit.reactnative

import org.servo.servokit.androidhost.ContextMenuElementInformation
import org.servo.servokit.androidhost.ServoHostEvent

import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.WritableArray
import com.facebook.react.bridge.WritableMap
import com.facebook.react.uimanager.events.Event
import org.json.JSONArray
import org.json.JSONObject

internal enum class ContextMenuItemKind {
  ITEM,
  SEPARATOR;

  fun toEventValue(): String =
    when (this) {
      ITEM -> "item"
      SEPARATOR -> "separator"
    }
}

internal enum class ContextMenuItemSource {
  SERVO,
  APP;

  fun toEventValue(): String =
    when (this) {
      SERVO -> "servo"
      APP -> "app"
    }
}

internal data class ReactNativeContextMenuItem(
  val kind: ContextMenuItemKind,
  val source: ContextMenuItemSource,
  val label: String? = null,
  val action: String? = null,
  val enabled: Boolean = true
)

internal const val CREATE_NEW_WEBVIEW_REQUESTED_EVENT_NAME = "topCreateNewWebViewRequested"

internal data class CreateNewWebViewRequestedEventPayload(
  val parentWebViewId: String,
  val parentUrl: String?,
  val targetUrl: String?,
  val windowFeatures: String?,
  val policy: String
)

private fun writableArrayOfStrings(values: List<String>): WritableArray =
  Arguments.createArray().apply {
    values.forEach(::pushString)
  }

private fun jsonOfContextMenuElementInformation(element: ContextMenuElementInformation): String =
  JSONObject()
    .put("isLink", element.isLink)
    .put("isImage", element.isImage)
    .put("isEditableText", element.isEditableText)
    .put("hasSelection", element.hasSelection)
    .put("linkUrl", element.linkUrl)
    .put("imageUrl", element.imageUrl)
    .put("contextType", element.contextType)
    .toString()

private fun jsonOfContextMenuItem(item: ReactNativeContextMenuItem): JSONObject =
  JSONObject()
    .put("type", item.kind.toEventValue())
    .put("source", item.source.toEventValue())
    .put("label", item.label)
    .put("action", item.action)
    .put("enabled", item.enabled)

private fun jsonOfContextMenuItems(items: List<ReactNativeContextMenuItem>): String =
  JSONArray().apply {
    items.forEach { item ->
      put(jsonOfContextMenuItem(item))
    }
  }.toString()

internal fun createNewWebViewRequestedEventPayload(
  event: ServoHostEvent.PopupRequested
): CreateNewWebViewRequestedEventPayload =
  CreateNewWebViewRequestedEventPayload(
    parentWebViewId = event.parentWebViewId,
    parentUrl = event.parentUrl,
    targetUrl = event.targetUrl,
    windowFeatures = event.windowFeatures,
    policy = event.policy
  )

private fun writableMapOfCreateNewWebViewRequestedEvent(
  payload: CreateNewWebViewRequestedEventPayload
): WritableMap =
  Arguments.createMap().apply {
    putString("parentWebViewId", payload.parentWebViewId)
    putString("parentUrl", payload.parentUrl)
    putString("targetUrl", payload.targetUrl)
    putString("windowFeatures", payload.windowFeatures)
    putString("policy", payload.policy)
  }

class ServoUrlChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val url: String
) : Event<ServoUrlChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topUrlChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("url", url)
    }
}

class ServoPageTitleChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val title: String?
) : Event<ServoPageTitleChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topPageTitleChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("title", title)
    }
}

class ServoStatusTextChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val status: String?
) : Event<ServoStatusTextChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topStatusTextChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("status", status)
    }
}

class ServoLoadStatusChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val status: String
) : Event<ServoLoadStatusChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topLoadStatusChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("status", status)
    }
}

class ServoHistoryChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val entries: List<String>,
  private val current: Int,
  private val canGoBack: Boolean,
  private val canGoForward: Boolean
) : Event<ServoHistoryChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topHistoryChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putArray("entries", writableArrayOfStrings(entries))
      putInt("current", current)
      putBoolean("canGoBack", canGoBack)
      putBoolean("canGoForward", canGoForward)
    }
}

class ServoFocusChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val isFocused: Boolean
) : Event<ServoFocusChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topFocusChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putBoolean("isFocused", isFocused)
    }
}

class ServoCursorChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val cursor: String
) : Event<ServoCursorChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topCursorChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("cursor", cursor)
    }
}

class ServoFullscreenChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val isFullscreen: Boolean
) : Event<ServoFullscreenChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topFullscreenChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putBoolean("isFullscreen", isFullscreen)
    }
}

class ServoClosedEvent(
  surfaceId: Int,
  viewId: Int
) : Event<ServoClosedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topClosed"

  override fun getEventData(): WritableMap = Arguments.createMap()
}

class ServoCrashedEvent(
  surfaceId: Int,
  viewId: Int,
  private val reason: String,
  private val backtrace: String?
) : Event<ServoCrashedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topCrashed"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("reason", reason)
      putString("backtrace", backtrace)
    }
}

class ServoErrorEvent(
  surfaceId: Int,
  viewId: Int,
  private val code: Int,
  private val message: String
) : Event<ServoErrorEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topError"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putInt("code", code)
      putString("message", message)
    }
}

class ServoJavaScriptEvaluationResultEvent(
  surfaceId: Int,
  viewId: Int,
  private val evaluationId: String,
  private val ok: Boolean,
  private val valueJson: String?,
  private val errorType: String?
) : Event<ServoJavaScriptEvaluationResultEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topJavaScriptEvaluationResult"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("evaluationId", evaluationId)
      putBoolean("ok", ok)
      putString("valueJson", valueJson)
      putString("errorType", errorType)
    }
}

class ServoControllerReadyEvent(
  surfaceId: Int,
  viewId: Int,
  private val controllerHandle: String
) : Event<ServoControllerReadyEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topControllerReady"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("controllerHandle", controllerHandle)
    }
}

class ServoShouldStartLoadWithRequestRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val navigationId: String,
  private val url: String
) : Event<ServoShouldStartLoadWithRequestRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topShouldStartLoadWithRequestRequested"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("navigationId", navigationId)
      putString("url", url)
    }
}

internal class ServoCreateNewWebViewRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val payload: CreateNewWebViewRequestedEventPayload
) : Event<ServoCreateNewWebViewRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = CREATE_NEW_WEBVIEW_REQUESTED_EVENT_NAME

  override fun getEventData(): WritableMap =
    writableMapOfCreateNewWebViewRequestedEvent(payload)
}

class ServoJavaScriptDialogRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val dialogId: String,
  private val kind: String,
  private val message: String,
  private val defaultValue: String?
) : Event<ServoJavaScriptDialogRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topJavaScriptDialogRequested"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("dialogId", dialogId)
      putString("kind", kind)
      putString("message", message)
      putString("defaultValue", defaultValue)
    }
}

class ServoJavaScriptDialogDismissedEvent(
  surfaceId: Int,
  viewId: Int,
  private val dialogId: String
) : Event<ServoJavaScriptDialogDismissedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topJavaScriptDialogDismissed"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("dialogId", dialogId)
    }
}

internal class ServoContextMenuRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val contextMenuId: String,
  private val element: ContextMenuElementInformation,
  private val servoItems: List<ReactNativeContextMenuItem>
) : Event<ServoContextMenuRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topContextMenuRequested"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("contextMenuId", contextMenuId)
      putString("elementJson", jsonOfContextMenuElementInformation(element))
      putString("servoItemsJson", jsonOfContextMenuItems(servoItems))
    }
}

internal class ServoContextMenuItemSelectedEvent(
  surfaceId: Int,
  viewId: Int,
  private val item: ReactNativeContextMenuItem,
  private val element: ContextMenuElementInformation
) : Event<ServoContextMenuItemSelectedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topContextMenuItemSelected"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("itemJson", jsonOfContextMenuItem(item).toString())
      putString("elementJson", jsonOfContextMenuElementInformation(element))
    }
}
