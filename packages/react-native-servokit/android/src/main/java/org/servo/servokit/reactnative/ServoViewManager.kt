package org.servo.servokit.reactnative

import com.facebook.react.bridge.ReadableArray
import com.facebook.react.bridge.ReadableMap
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.ViewManagerDelegate
import com.facebook.react.viewmanagers.ServoViewManagerDelegate
import com.facebook.react.viewmanagers.ServoViewManagerInterface

@ReactModule(name = ServoViewManager.NAME)
class ServoViewManager : SimpleViewManager<ServoView>(),
  ServoViewManagerInterface<ServoView> {
  private val mDelegate: ViewManagerDelegate<ServoView>

  init {
    mDelegate = ServoViewManagerDelegate(this)
  }

  override fun getDelegate(): ViewManagerDelegate<ServoView>? {
    return mDelegate
  }

  override fun getName(): String {
    return NAME
  }

  public override fun createViewInstance(context: ThemedReactContext): ServoView {
    return ServoView(context)
  }

  override fun onDropViewInstance(view: ServoView) {
    view.dispose()
    super.onDropViewInstance(view)
  }

  override fun setUrl(view: ServoView?, url: String?) {
    val actualView = view ?: return
    val actualUrl = url ?: return
    actualView.loadUrl(actualUrl)
  }

  override fun setUseReactNativeJavaScriptDialogs(view: ServoView?, value: Boolean) {
    val actualView = view ?: return
    actualView.useReactNativeJavaScriptDialogs = value
  }

  override fun setUseReactNativeContextMenus(view: ServoView?, value: Boolean) {
    val actualView = view ?: return
    actualView.useReactNativeContextMenus = value
  }

  override fun setUseReactNativeOnShouldStartLoadWithRequest(view: ServoView?, value: Boolean) {
    val actualView = view ?: return
    actualView.useReactNativeOnShouldStartLoadWithRequest = value
  }

  override fun sendControllerCommand(view: ServoView, commandJson: String) {
    view.sendControllerCommand(commandJson)
  }

  override fun getCommandsMap(): MutableMap<String, Int> {
    return mutableMapOf(
      COMMAND_SEND_CONTROLLER_COMMAND_NAME to COMMAND_SEND_CONTROLLER_COMMAND,
      COMMAND_SHOW_CONTEXT_MENU_NAME to COMMAND_SHOW_CONTEXT_MENU
    )
  }

  override fun receiveCommand(view: ServoView, commandId: Int, args: ReadableArray?) {
    when (commandId) {
      COMMAND_SEND_CONTROLLER_COMMAND -> handleSendControllerCommand(view, args)
      COMMAND_SHOW_CONTEXT_MENU -> handleShowContextMenuCommand(view, args)
      else -> super.receiveCommand(view, commandId, args)
    }
  }

  override fun receiveCommand(view: ServoView, commandId: String, args: ReadableArray?) {
    when (commandId) {
      COMMAND_SEND_CONTROLLER_COMMAND_NAME,
      COMMAND_SEND_CONTROLLER_COMMAND.toString() -> handleSendControllerCommand(view, args)
      COMMAND_SHOW_CONTEXT_MENU_NAME,
      COMMAND_SHOW_CONTEXT_MENU.toString() -> handleShowContextMenuCommand(view, args)
      else -> super.receiveCommand(view, commandId, args)
    }
  }

  private fun handleSendControllerCommand(view: ServoView, args: ReadableArray?) {
    val commandJson = args?.getString(0) ?: return
    sendControllerCommand(view, commandJson)
  }

  private fun handleShowContextMenuCommand(view: ServoView, args: ReadableArray?) {
    val contextMenuId = args?.getString(0) ?: return
    view.showReactNativeContextMenu(contextMenuId, parseInjectedContextMenuItems(args.getArray(1)))
  }

  private fun parseInjectedContextMenuItems(items: ReadableArray?): List<ReactNativeContextMenuItem> =
    buildList {
      val count = items?.size() ?: 0
      for (index in 0 until count) {
        val item = items?.getMap(index) ?: continue
        parseInjectedContextMenuItem(item)?.let(::add)
      }
    }

  private fun parseInjectedContextMenuItem(item: ReadableMap): ReactNativeContextMenuItem? {
    val type = item.getString("type") ?: "item"
    return if (type == "separator") {
      ReactNativeContextMenuItem(
        kind = ContextMenuItemKind.SEPARATOR,
        source = ContextMenuItemSource.APP
      )
    } else {
      val label = item.getString("label") ?: return null
      val action = item.getString("action") ?: return null
      val enabled =
        if (item.hasKey("enabled") && !item.isNull("enabled")) {
          item.getBoolean("enabled")
        } else {
          true
        }
      ReactNativeContextMenuItem(
        kind = ContextMenuItemKind.ITEM,
        source = ContextMenuItemSource.APP,
        label = label,
        action = action,
        enabled = enabled
      )
    }
  }

  companion object {
    const val NAME = "ServoView"
    private const val COMMAND_SEND_CONTROLLER_COMMAND = 1
    private const val COMMAND_SHOW_CONTEXT_MENU = 2
    private const val COMMAND_SEND_CONTROLLER_COMMAND_NAME = "sendControllerCommand"
    private const val COMMAND_SHOW_CONTEXT_MENU_NAME = "showContextMenu"
  }
}
