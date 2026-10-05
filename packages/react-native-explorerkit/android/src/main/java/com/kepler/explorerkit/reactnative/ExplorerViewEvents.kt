package com.kepler.explorerkit.reactnative

import com.kepler.explorerkit.androidhost.ContextMenuElementInformation
import com.kepler.explorerkit.androidhost.ExplorerHostEvent

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
  event: ExplorerHostEvent.PopupRequested
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

class ExplorerUrlChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val url: String
) : Event<ExplorerUrlChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topUrlChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("url", url)
    }
}

class ExplorerPageTitleChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val title: String?
) : Event<ExplorerPageTitleChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topPageTitleChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("title", title)
    }
}

class ExplorerStatusTextChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val status: String?
) : Event<ExplorerStatusTextChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topStatusTextChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("status", status)
    }
}

class ExplorerLoadStatusChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val status: String
) : Event<ExplorerLoadStatusChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topLoadStatusChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("status", status)
    }
}

class ExplorerHistoryChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val entries: List<String>,
  private val current: Int,
  private val canGoBack: Boolean,
  private val canGoForward: Boolean
) : Event<ExplorerHistoryChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topHistoryChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putArray("entries", writableArrayOfStrings(entries))
      putInt("current", current)
      putBoolean("canGoBack", canGoBack)
      putBoolean("canGoForward", canGoForward)
    }
}

class ExplorerFocusChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val isFocused: Boolean
) : Event<ExplorerFocusChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topFocusChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putBoolean("isFocused", isFocused)
    }
}

class ExplorerCursorChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val cursor: String
) : Event<ExplorerCursorChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topCursorChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("cursor", cursor)
    }
}

class ExplorerFullscreenChangedEvent(
  surfaceId: Int,
  viewId: Int,
  private val isFullscreen: Boolean
) : Event<ExplorerFullscreenChangedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topFullscreenChanged"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putBoolean("isFullscreen", isFullscreen)
    }
}

class ExplorerClosedEvent(
  surfaceId: Int,
  viewId: Int
) : Event<ExplorerClosedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topClosed"

  override fun getEventData(): WritableMap = Arguments.createMap()
}

class ExplorerCrashedEvent(
  surfaceId: Int,
  viewId: Int,
  private val reason: String,
  private val backtrace: String?
) : Event<ExplorerCrashedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topCrashed"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("reason", reason)
      putString("backtrace", backtrace)
    }
}

class ExplorerErrorEvent(
  surfaceId: Int,
  viewId: Int,
  private val code: Int,
  private val message: String
) : Event<ExplorerErrorEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topError"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putInt("code", code)
      putString("message", message)
    }
}

class ExplorerJavaScriptEvaluationResultEvent(
  surfaceId: Int,
  viewId: Int,
  private val evaluationId: String,
  private val ok: Boolean,
  private val valueJson: String?,
  private val errorType: String?
) : Event<ExplorerJavaScriptEvaluationResultEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topJavaScriptEvaluationResult"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("evaluationId", evaluationId)
      putBoolean("ok", ok)
      putString("valueJson", valueJson)
      putString("errorType", errorType)
    }
}

class ExplorerControllerReadyEvent(
  surfaceId: Int,
  viewId: Int,
  private val controllerHandle: String
) : Event<ExplorerControllerReadyEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topControllerReady"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("controllerHandle", controllerHandle)
    }
}

class ExplorerShouldStartLoadWithRequestRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val navigationId: String,
  private val url: String
) : Event<ExplorerShouldStartLoadWithRequestRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topShouldStartLoadWithRequestRequested"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("navigationId", navigationId)
      putString("url", url)
    }
}

internal class ExplorerCreateNewWebViewRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val payload: CreateNewWebViewRequestedEventPayload
) : Event<ExplorerCreateNewWebViewRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = CREATE_NEW_WEBVIEW_REQUESTED_EVENT_NAME

  override fun getEventData(): WritableMap =
    writableMapOfCreateNewWebViewRequestedEvent(payload)
}

class ExplorerJavaScriptDialogRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val dialogId: String,
  private val kind: String,
  private val message: String,
  private val defaultValue: String?
) : Event<ExplorerJavaScriptDialogRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topJavaScriptDialogRequested"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("dialogId", dialogId)
      putString("kind", kind)
      putString("message", message)
      putString("defaultValue", defaultValue)
    }
}

class ExplorerJavaScriptDialogDismissedEvent(
  surfaceId: Int,
  viewId: Int,
  private val dialogId: String
) : Event<ExplorerJavaScriptDialogDismissedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topJavaScriptDialogDismissed"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("dialogId", dialogId)
    }
}

internal class ExplorerContextMenuRequestedEvent(
  surfaceId: Int,
  viewId: Int,
  private val contextMenuId: String,
  private val element: ContextMenuElementInformation,
  private val servoItems: List<ReactNativeContextMenuItem>
) : Event<ExplorerContextMenuRequestedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topContextMenuRequested"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("contextMenuId", contextMenuId)
      putString("elementJson", jsonOfContextMenuElementInformation(element))
      putString("servoItemsJson", jsonOfContextMenuItems(servoItems))
    }
}

internal class ExplorerContextMenuItemSelectedEvent(
  surfaceId: Int,
  viewId: Int,
  private val item: ReactNativeContextMenuItem,
  private val element: ContextMenuElementInformation
) : Event<ExplorerContextMenuItemSelectedEvent>(surfaceId, viewId) {
  override fun getEventName(): String = "topContextMenuItemSelected"

  override fun getEventData(): WritableMap =
    Arguments.createMap().apply {
      putString("itemJson", jsonOfContextMenuItem(item).toString())
      putString("elementJson", jsonOfContextMenuElementInformation(element))
    }
}
