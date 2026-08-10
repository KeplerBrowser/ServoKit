package org.servo.servokit.androidhost

import android.view.Surface
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class ServoViewBindingTest {
  @Test
  fun returnsPlainHostEventsInOrderForSuccessfulLoads() {
    val events =
      listOf(
        ServoHostEvent.UrlChanged("https://example.com/"),
        ServoHostEvent.LoadStatusChanged("Started")
      )
    val host = FakeServoHost(loadStatus = 0, loadEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.loadUrl("example.com"))
  }

  @Test
  fun returnsPlainNativeFailureEvents() {
    val events =
      listOf(ServoHostEvent.Error("https:///", 1, "invalid url: empty host"))
    val host = FakeServoHost(loadStatus = 1, loadEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.loadUrl("https:///"))
  }

  @Test
  fun returnsNativeSurfaceAttachEvents() {
    val events: List<ServoHostEvent> = listOf(ServoHostEvent.SurfaceAttached(640, 480))
    val host = FakeServoHost(attachEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.attachSurface(fakeSurface(), 640, 480, 2f))
    assertEquals(1, host.attachCalls)
    assertEquals(SurfaceMetrics(640, 480, 2f), host.lastAttachedSurfaceMetrics)
    assertEquals(0, host.resizeCalls)
    assertEquals(0, host.detachCalls)
  }

  @Test
  fun returnsNativeSurfaceResizeEvents() {
    val events: List<ServoHostEvent> = listOf(ServoHostEvent.SurfaceResized(800, 600))
    val host = FakeServoHost(resizeEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.resizeSurface(800, 600, 2f))
    assertEquals(0, host.attachCalls)
    assertEquals(1, host.resizeCalls)
    assertEquals(SurfaceMetrics(800, 600, 2f), host.lastResizedSurfaceMetrics)
    assertEquals(0, host.detachCalls)
  }

  @Test
  fun returnsNativeSurfaceDetachEvents() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.SurfaceDetached)
    val host = FakeServoHost(detachEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.detachSurface())
    assertEquals(0, host.attachCalls)
    assertEquals(0, host.resizeCalls)
    assertEquals(1, host.detachCalls)
  }

  @Test
  fun rejectsNegativeSurfaceAttachDimensions() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertThrows(IllegalArgumentException::class.java) {
      binding.attachSurface(fakeSurface(), -1, 480, 2f)
    }
    assertEquals(0, host.attachCalls)
  }

  @Test
  fun rejectsNegativeSurfaceResizeDimensions() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertThrows(IllegalArgumentException::class.java) {
      binding.resizeSurface(800, -1, 2f)
    }
    assertEquals(0, host.resizeCalls)
  }

  @Test
  fun rejectsNonPositiveSurfaceDensity() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertThrows(IllegalArgumentException::class.java) {
      binding.attachSurface(fakeSurface(), 640, 480, 0f)
    }
    assertEquals(0, host.attachCalls)
  }

  @Test
  fun returnsEmptyListsWhenTheHostHasNoImmediateEvents() {
    val host = FakeServoHost(loadStatus = 0)
    val binding = ServoViewBinding(host)

    assertPlainEvents(emptyList(), binding.loadUrl("https://example.com"))
  }

  @Test
  fun drainsUpdateEventsFromPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.LoadStatusChanged("Complete"))
    val host = FakeServoHost(updateEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.performUpdates())
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsTouchEventsToTheHost() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.UrlChanged("https://example.com/"))
    val host = FakeServoHost(touchEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.dispatchTouchEvent(TOUCH_EVENT_DOWN, 4, 12.5f, 30.25f))
    assertEquals(1, host.touchDispatchCalls)
    assertEquals(TouchDispatch(TOUCH_EVENT_DOWN, 4, 12.5f, 30.25f), host.lastTouchDispatch)
  }

  @Test
  fun forwardsContextMenuTriggersToTheHost() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.ContextMenuDismissed("context-menu-1"))
    val host = FakeServoHost(triggerContextMenuEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.triggerContextMenu(25f, 50f))
    assertEquals(ContextMenuTrigger(25f, 50f), host.lastContextMenuTrigger)
  }

  @Test
  fun forwardsReloadToTheHost() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.LoadStatusChanged("Started"))
    val host = FakeServoHost(reloadEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.reload())
    assertEquals(1, host.reloadCalls)
  }

  @Test
  fun forwardsGoBackToTheHost() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertPlainEvents(emptyList(), binding.goBack())
    assertEquals(1, host.goBackCalls)
  }

  @Test
  fun forwardsGoForwardToTheHost() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertPlainEvents(emptyList(), binding.goForward())
    assertEquals(1, host.goForwardCalls)
  }

  @Test
  fun forwardsFocusToTheHost() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.FocusChanged(true))
    val host = FakeServoHost(focusEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.focus())
    assertEquals(1, host.focusCalls)
  }

  @Test
  fun forwardsBlurToTheHost() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.FocusChanged(false))
    val host = FakeServoHost(blurEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.blur())
    assertEquals(1, host.blurCalls)
  }

  @Test
  fun forwardsMountedControllerCommandsThroughTheCurrentControllerHandle() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertEquals(0, binding.sendControllerCommand("{\"command\":\"reload\"}"))
    assertEquals(
      ControllerCommandEnvelope("controller-handle", "{\"command\":\"reload\"}"),
      host.lastControllerCommand
    )
  }

  @Test
  fun mountedControllerCommandsReturnNullPointerStatusWithoutAControllerHandle() {
    val host = FakeServoHost(controllerHandle = null)
    val binding = ServoViewBinding(host)

    assertEquals(2, binding.sendControllerCommand("{\"command\":\"reload\"}"))
    assertEquals(null, host.lastControllerCommand)
  }

  @Test
  fun forwardsSimpleDialogResolutionToTheHost() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.LoadStatusChanged("Complete"))
    val host = FakeServoHost(resolveSimpleDialogEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(
      events,
      binding.resolveSimpleDialog("dialog-1", SIMPLE_DIALOG_ACTION_CONFIRM, "servo")
    )
    assertEquals(SimpleDialogResolution("dialog-1", SIMPLE_DIALOG_ACTION_CONFIRM, "servo"), host.lastResolvedSimpleDialog)
  }

  @Test
  fun forwardsSelectElementResolutionThroughPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.LoadStatusChanged("Complete"))
    val host = FakeServoHost(resolveSelectElementEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.resolveSelectElement("select-1", intArrayOf(4, 9)))
    assertEquals(SelectElementResolution("select-1", listOf(4, 9)), host.lastResolvedSelectElement)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsContextMenuResolutionThroughPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.ContextMenuDismissed("context-menu-1"))
    val host = FakeServoHost(resolveContextMenuEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.resolveContextMenu("context-menu-1", "copy-link"))
    assertEquals(ContextMenuResolution("context-menu-1", "copy-link"), host.lastResolvedContextMenu)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsControllerContextMenuResolutionWithoutPerformUpdates() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertPlainEvents(
      emptyList(),
      binding.resolveContextMenu("controller-1", "context-menu-1", "copy-link")
    )
    assertEquals(
      ControllerContextMenuResolution("controller-1", "context-menu-1", "copy-link"),
      host.lastControllerResolvedContextMenu
    )
    assertEquals(0, host.performUpdatesCalls)
  }

  @Test
  fun forwardsContextMenuDismissalThroughPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.ContextMenuDismissed("context-menu-2"))
    val host = FakeServoHost(dismissContextMenuEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.dismissContextMenu("context-menu-2"))
    assertEquals("context-menu-2", host.lastDismissedContextMenuId)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsControllerContextMenuDismissalWithoutPerformUpdates() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertPlainEvents(
      emptyList(),
      binding.dismissContextMenu("controller-2", "context-menu-2")
    )
    assertEquals(
      ControllerContextMenuDismissal("controller-2", "context-menu-2"),
      host.lastControllerDismissedContextMenu
    )
    assertEquals(0, host.performUpdatesCalls)
  }

  @Test
  fun forwardsFilePickerResolutionThroughPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.FilePickerDismissed("file-picker-1"))
    val host = FakeServoHost(resolveFilePickerEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.resolveFilePicker("file-picker-1", arrayOf("/cache/one.txt")))
    assertEquals(
      FilePickerResolution("file-picker-1", listOf("/cache/one.txt")),
      host.lastResolvedFilePicker
    )
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsFilePickerDismissalThroughPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.FilePickerDismissed("file-picker-2"))
    val host = FakeServoHost(dismissFilePickerEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.dismissFilePicker("file-picker-2"))
    assertEquals("file-picker-2", host.lastDismissedFilePickerId)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun exposesAndResolvesPendingPermissionRequests() {
    val pendingRequest = ServoHostEvent.PermissionRequested("geolocation", "http://127.0.0.1:3000")
    val events = listOf<ServoHostEvent>(ServoHostEvent.LoadStatusChanged("Complete"))
    val host = FakeServoHost(pendingPermissionRequest = pendingRequest, resolvePermissionEvents = events)
    val binding = ServoViewBinding(host)

    assertEquals(true, binding.hasPendingPermissionRequest())
    assertEquals(pendingRequest, binding.getPendingPermissionRequest())
    assertPlainEvents(events, binding.resolvePermission(true))
    assertEquals(true, host.lastResolvedPermissionAllow)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsNavigationRequestResolutionThroughPerformUpdates() {
    val events = listOf<ServoHostEvent>(ServoHostEvent.LoadStatusChanged("Started"))
    val host = FakeServoHost(resolveNavigationRequestEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.resolveNavigationRequest("navigation-1", false))
    assertEquals(NavigationRequestResolution("navigation-1", false), host.lastResolvedNavigationRequest)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsImeCompositionThroughPerformUpdates() {
    val events =
      listOf<ServoHostEvent>(
        ServoHostEvent.InputMethodRequested(
          inputMethodId = "ime-1",
          type = "text",
          text = "Servo",
          insertionPoint = 5,
          multiline = false,
          allowVirtualKeyboard = true
        )
      )
    val host = FakeServoHost(updateEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.dispatchImeComposition(IME_COMPOSITION_END, "Servo"))
    assertEquals(ImeDispatch(IME_COMPOSITION_END, "Servo"), host.lastImeDispatch)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsKeyboardKeysAndImeDismissalToTheHost() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    assertPlainEvents(emptyList(), binding.dispatchKeyboardKey(KEYBOARD_KEY_BACKSPACE))
    assertPlainEvents(emptyList(), binding.dismissInputMethod())
    assertEquals(KEYBOARD_KEY_BACKSPACE, host.lastKeyboardKey)
    assertEquals(1, host.dismissInputMethodCalls)
  }

  @Test
  fun navigationCommandsReturnEmptyListAfterDispose() {
    val host = FakeServoHost()
    val binding = ServoViewBinding(host)

    binding.dispose()

    assertPlainEvents(emptyList(), binding.reload())
    assertPlainEvents(emptyList(), binding.goBack())
    assertPlainEvents(emptyList(), binding.goForward())
    assertPlainEvents(emptyList(), binding.focus())
    assertPlainEvents(emptyList(), binding.blur())
    assertEquals(0, host.reloadCalls)
    assertEquals(0, host.goBackCalls)
    assertEquals(0, host.goForwardCalls)
    assertEquals(0, host.focusCalls)
    assertEquals(0, host.blurCalls)
  }

  @Test
  fun disposingTheBindingOnlyDestroysTheNativeHostOnce() {
    val host = FakeServoHost(loadStatus = 0)
    val binding = ServoViewBinding(host)

    binding.dispose()
    binding.dispose()

    assertEquals(1, host.destroyCalls)
  }

  @Test
  fun loadUrlAfterDisposeReturnsASingleErrorEvent() {
    val host = FakeServoHost(loadStatus = 0)
    val binding = ServoViewBinding(host)

    binding.dispose()

    assertPlainEvents(
      listOf(ServoHostEvent.Error("https://example.com", 2, "binding has been disposed")),
      binding.loadUrl("https://example.com")
    )
  }

  @Test
  fun preservesServoStyleDelegatePayloads() {
    val events =
      listOf<ServoHostEvent>(
        ServoHostEvent.PageTitleChanged("Example Domain"),
        ServoHostEvent.StatusTextChanged("Done"),
        ServoHostEvent.HistoryChanged(
          entries = listOf("https://example.com/", "https://www.rust-lang.org/"),
          current = 1,
          canGoBack = true,
          canGoForward = false
        ),
        ServoHostEvent.Crashed(
          url = "https://example.com/",
          reason = "renderer panic",
          backtrace = "stack"
        ),
        ServoHostEvent.SimpleDialogRequested(
          dialogId = "dialog-1",
          kind = "prompt",
          message = "Name?",
          defaultValue = "Servo"
        ),
        ServoHostEvent.SimpleDialogDismissed("dialog-1"),
        ServoHostEvent.InputMethodRequested(
          inputMethodId = "ime-1",
          type = "email",
          text = "servo@example.com",
          insertionPoint = 17,
          multiline = false,
          allowVirtualKeyboard = true
        ),
        ServoHostEvent.InputMethodDismissed("ime-1"),
        ServoHostEvent.ContextMenuRequested(
          contextMenuId = "context-menu-1",
          x = 12,
          y = 24,
          width = 32,
          height = 48,
          elementInfo =
            ContextMenuElementInformation(
              isLink = true,
              isImage = true,
              isEditableText = false,
              hasSelection = false,
              linkUrl = "https://example.com/",
              imageUrl = "https://example.com/image.png",
              contextType = "link"
            ),
          items =
            listOf(
              ContextMenuItem.Item("Copy link", "copy-link", true),
              ContextMenuItem.Separator,
              ContextMenuItem.Item("Reload", "reload", false)
            )
        ),
        ServoHostEvent.ContextMenuDismissed("context-menu-1"),
        ServoHostEvent.FilePickerRequested(
          filePickerId = "file-picker-1",
          currentPaths = listOf("/cache/one.txt"),
          filterPatterns = listOf("txt", "png"),
          allowSelectMultiple = true
        ),
        ServoHostEvent.FilePickerDismissed("file-picker-1"),
        ServoHostEvent.PermissionRequested("geolocation", "http://127.0.0.1:3000"),
        ServoHostEvent.FocusChanged(true),
        ServoHostEvent.CursorChanged("pointer"),
        ServoHostEvent.FullscreenChanged(true),
        ServoHostEvent.Closed
      )
    val host = FakeServoHost(updateEvents = events)
    val binding = ServoViewBinding(host)

    assertPlainEvents(events, binding.performUpdates())
  }
}

private fun assertPlainEvents(expected: List<ServoHostEvent>, actual: List<ServoHostEvent>) {
  assertEquals(expected, actual)
  assertEquals(expected.javaClass, actual.javaClass)
}

private fun fakeSurface(): Surface {
  val unsafeClass = Class.forName("sun.misc.Unsafe")
  val field = unsafeClass.getDeclaredField("theUnsafe")
  field.isAccessible = true
  val unsafe = field.get(null)
  val allocateInstance = unsafeClass.getMethod("allocateInstance", Class::class.java)
  return allocateInstance.invoke(unsafe, Surface::class.java) as Surface
}

private class FakeServoHost(
  private val controllerHandle: String? = "controller-handle",
  private val loadStatus: Int = 0,
  private val loadEvents: List<ServoHostEvent> = emptyList(),
  private val attachEvents: List<ServoHostEvent> = emptyList(),
  private val resizeEvents: List<ServoHostEvent> = emptyList(),
  private val detachEvents: List<ServoHostEvent> = emptyList(),
  private val touchEvents: List<ServoHostEvent> = emptyList(),
  private val updateEvents: List<ServoHostEvent> = emptyList(),
  private val reloadEvents: List<ServoHostEvent> = emptyList(),
  private val goBackEvents: List<ServoHostEvent> = emptyList(),
  private val goForwardEvents: List<ServoHostEvent> = emptyList(),
  private val focusEvents: List<ServoHostEvent> = emptyList(),
  private val blurEvents: List<ServoHostEvent> = emptyList(),
  private val resolveSimpleDialogEvents: List<ServoHostEvent> = emptyList(),
  private val resolveSelectElementEvents: List<ServoHostEvent> = emptyList(),
  private val resolveContextMenuEvents: List<ServoHostEvent> = emptyList(),
  private val dismissContextMenuEvents: List<ServoHostEvent> = emptyList(),
  private val resolveFilePickerEvents: List<ServoHostEvent> = emptyList(),
  private val dismissFilePickerEvents: List<ServoHostEvent> = emptyList(),
  private val triggerContextMenuEvents: List<ServoHostEvent> = emptyList(),
  private val pendingPermissionRequest: ServoHostEvent.PermissionRequested? = null,
  private val resolvePermissionEvents: List<ServoHostEvent> = emptyList(),
  private val resolveNavigationRequestEvents: List<ServoHostEvent> = emptyList()
) : ServoHost {
  var destroyCalls = 0
    private set
  var attachCalls = 0
    private set
  var resizeCalls = 0
    private set
  var detachCalls = 0
    private set
  var performUpdatesCalls = 0
    private set
  var touchDispatchCalls = 0
    private set
  var reloadCalls = 0
    private set
  var goBackCalls = 0
    private set
  var goForwardCalls = 0
    private set
  var focusCalls = 0
    private set
  var blurCalls = 0
    private set
  var dismissInputMethodCalls = 0
    private set
  var lastAttachedSurfaceMetrics: SurfaceMetrics? = null
    private set
  var lastResizedSurfaceMetrics: SurfaceMetrics? = null
    private set
  var lastTouchDispatch: TouchDispatch? = null
    private set
  var lastResolvedSimpleDialog: SimpleDialogResolution? = null
    private set
  var lastResolvedSelectElement: SelectElementResolution? = null
    private set
  var lastResolvedContextMenu: ContextMenuResolution? = null
    private set
  var lastControllerResolvedContextMenu: ControllerContextMenuResolution? = null
    private set
  var lastControllerCommand: ControllerCommandEnvelope? = null
    private set
  var lastContextMenuTrigger: ContextMenuTrigger? = null
    private set
  var lastResolvedFilePicker: FilePickerResolution? = null
    private set
  var lastDismissedContextMenuId: String? = null
    private set
  var lastControllerDismissedContextMenu: ControllerContextMenuDismissal? = null
    private set
  var lastDismissedFilePickerId: String? = null
    private set
  var lastImeDispatch: ImeDispatch? = null
    private set
  var lastKeyboardKey: Int? = null
    private set
  var lastResolvedPermissionAllow: Boolean? = null
    private set
  var lastResolvedNavigationRequest: NavigationRequestResolution? = null
    private set
  private var pendingEvents: List<ServoHostEvent> = emptyList()

  override fun controllerHandle(): String? = controllerHandle

  override fun loadUrl(input: String): Int {
    pendingEvents = loadEvents
    return loadStatus
  }

  override fun reload() {
    reloadCalls += 1
    pendingEvents = reloadEvents
  }

  override fun goBack() {
    goBackCalls += 1
    pendingEvents = goBackEvents
  }

  override fun goForward() {
    goForwardCalls += 1
    pendingEvents = goForwardEvents
  }

  override fun focus() {
    focusCalls += 1
    pendingEvents = focusEvents
  }

  override fun blur() {
    blurCalls += 1
    pendingEvents = blurEvents
  }

  override fun resolveSimpleDialog(dialogId: String, action: Int, promptValue: String?) {
    lastResolvedSimpleDialog = SimpleDialogResolution(dialogId, action, promptValue)
    pendingEvents = resolveSimpleDialogEvents
  }

  override fun resolveSelectElement(selectElementId: String, selectedOptions: IntArray) {
    lastResolvedSelectElement = SelectElementResolution(selectElementId, selectedOptions.toList())
  }

  override fun resolveContextMenu(contextMenuId: String, action: String) {
    lastResolvedContextMenu = ContextMenuResolution(contextMenuId, action)
  }

  override fun resolveContextMenu(
    controllerHandle: String,
    contextMenuId: String,
    action: String
  ): Int {
    lastControllerResolvedContextMenu =
      ControllerContextMenuResolution(controllerHandle, contextMenuId, action)
    return 0
  }

  override fun sendControllerCommand(controllerHandle: String, commandJson: String): Int {
    lastControllerCommand = ControllerCommandEnvelope(controllerHandle, commandJson)
    return 0
  }

  override fun dismissContextMenu(contextMenuId: String) {
    lastDismissedContextMenuId = contextMenuId
  }

  override fun dismissContextMenu(controllerHandle: String, contextMenuId: String): Int {
    lastControllerDismissedContextMenu =
      ControllerContextMenuDismissal(controllerHandle, contextMenuId)
    return 0
  }

  override fun resolveFilePicker(filePickerId: String, selectedPaths: Array<String>) {
    lastResolvedFilePicker = FilePickerResolution(filePickerId, selectedPaths.toList())
  }

  override fun dismissFilePicker(filePickerId: String) {
    lastDismissedFilePickerId = filePickerId
  }

  override fun hasPendingPermissionRequest(): Boolean = pendingPermissionRequest != null

  override fun getPendingPermissionRequest(): ServoHostEvent.PermissionRequested? = pendingPermissionRequest

  override fun resolvePermission(allow: Boolean) {
    lastResolvedPermissionAllow = allow
  }

  override fun resolveNavigationRequest(navigationId: String, allow: Boolean) {
    lastResolvedNavigationRequest = NavigationRequestResolution(navigationId, allow)
  }

  override fun dispatchImeComposition(state: Int, text: String) {
    lastImeDispatch = ImeDispatch(state, text)
  }

  override fun dismissInputMethod() {
    dismissInputMethodCalls += 1
  }

  override fun dispatchKeyboardKey(key: Int) {
    lastKeyboardKey = key
  }

  override fun attachSurface(surface: Surface, width: Int, height: Int, density: Float) {
    attachCalls += 1
    lastAttachedSurfaceMetrics = SurfaceMetrics(width, height, density)
    pendingEvents = attachEvents
  }

  override fun resizeSurface(width: Int, height: Int, density: Float) {
    resizeCalls += 1
    lastResizedSurfaceMetrics = SurfaceMetrics(width, height, density)
    pendingEvents = resizeEvents
  }

  override fun detachSurface() {
    detachCalls += 1
    pendingEvents = detachEvents
  }

  override fun performUpdates() {
    performUpdatesCalls += 1
    pendingEvents =
      when {
        lastResolvedFilePicker != null -> resolveFilePickerEvents
        lastDismissedContextMenuId != null -> dismissContextMenuEvents
        lastResolvedContextMenu != null -> resolveContextMenuEvents
        lastDismissedFilePickerId != null -> dismissFilePickerEvents
        lastResolvedPermissionAllow != null -> resolvePermissionEvents
        lastResolvedNavigationRequest != null -> resolveNavigationRequestEvents
        lastResolvedSelectElement != null -> resolveSelectElementEvents
        else -> updateEvents
      }
  }

  override fun dispatchTouchEvent(action: Int, pointerId: Int, x: Float, y: Float) {
    touchDispatchCalls += 1
    lastTouchDispatch = TouchDispatch(action, pointerId, x, y)
    pendingEvents = touchEvents
  }

  override fun triggerContextMenu(x: Float, y: Float) {
    lastContextMenuTrigger = ContextMenuTrigger(x, y)
    pendingEvents = triggerContextMenuEvents
  }

  override fun drainEvents(): List<ServoHostEvent> =
    pendingEvents.also {
      pendingEvents = emptyList()
    }

  override fun destroy() {
    destroyCalls += 1
  }
}

private data class TouchDispatch(
  val action: Int,
  val pointerId: Int,
  val x: Float,
  val y: Float
)

private data class ContextMenuTrigger(
  val x: Float,
  val y: Float
)

private data class SurfaceMetrics(
  val width: Int,
  val height: Int,
  val density: Float
)

private data class SimpleDialogResolution(
  val dialogId: String,
  val action: Int,
  val promptValue: String?
)

private data class SelectElementResolution(
  val selectElementId: String,
  val selectedOptions: List<Int>
)

private data class ContextMenuResolution(
  val contextMenuId: String,
  val action: String
)

private data class ControllerContextMenuResolution(
  val controllerHandle: String,
  val contextMenuId: String,
  val action: String
)

private data class ControllerCommandEnvelope(
  val controllerHandle: String,
  val commandJson: String
)

private data class ControllerContextMenuDismissal(
  val controllerHandle: String,
  val contextMenuId: String
)

private data class FilePickerResolution(
  val filePickerId: String,
  val selectedPaths: List<String>
)

private data class NavigationRequestResolution(
  val navigationId: String,
  val allow: Boolean
)

private data class ImeDispatch(
  val state: Int,
  val text: String
)
