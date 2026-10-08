package com.kepler.explorerkit.androidhost

import android.view.Surface
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class ExplorerViewBindingTest {
  @Test
  fun returnsPlainHostEventsInOrderForSuccessfulLoads() {
    val events =
      listOf(
        ExplorerHostEvent.UrlChanged("https://example.com/"),
        ExplorerHostEvent.LoadStatusChanged("Started")
      )
    val host = FakeExplorerHost(loadStatus = 0, loadEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.loadUrl("example.com"))
  }

  @Test
  fun returnsPlainNativeFailureEvents() {
    val events =
      listOf(ExplorerHostEvent.Error("https:///", 1, "invalid url: empty host"))
    val host = FakeExplorerHost(loadStatus = 1, loadEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.loadUrl("https:///"))
  }

  @Test
  fun returnsInitialNavigationErrorsBeforeAnySurfaceCall() {
    val error = ExplorerHostEvent.Error("https:///", 1, "invalid url: empty host")
    val host = FakeExplorerHost(loadStatus = 1, loadEvents = listOf(error))
    val events = mutableListOf<ExplorerHostEvent>()
    val coordinator = createSurfaceCoordinator(ExplorerViewBinding(host), events)

    coordinator.loadUrl("https:///")

    assertEquals(listOf(error), events)
    assertEquals(listOf("https:///"), host.loadedUrls)
    assertEquals(0, host.attachCalls)
    assertEquals(0, host.performUpdatesCalls)
  }

  @Test
  fun keepsNavigationBufferedAfterADetachErrorUntilRebind() {
    val error = ExplorerHostEvent.Error(null, 4, "surface detach failed")
    val host = FakeExplorerHost(detachEvents = listOf(error))
    val events = mutableListOf<ExplorerHostEvent>()
    val coordinator = createSurfaceCoordinator(ExplorerViewBinding(host), events)
    coordinator.loadUrl("https://example.com/initial")
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    coordinator.onSurfaceDestroyed()
    coordinator.loadUrl("https://example.com/detached")
    coordinator.loadUrl("https:///")

    assertEquals(listOf(error), events)
    assertEquals(listOf("https://example.com/initial"), host.loadedUrls)
    assertEquals(1, host.detachCalls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(listOf("https://example.com/initial", "https:///"), host.loadedUrls)
    assertEquals(2, host.attachCalls)
  }

  @Test
  fun doesNotForwardNavigationAfterDisposalBeforeFirstAttach() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)
    val events = mutableListOf<ExplorerHostEvent>()
    val coordinator = createSurfaceCoordinator(binding, events)
    coordinator.loadUrl("https://example.com/initial")

    coordinator.onViewDisposed()
    binding.dispose()
    coordinator.loadUrl("https://example.com/after-dispose")

    assertEquals(listOf("https://example.com/initial"), host.loadedUrls)
    assertEquals(emptyList<ExplorerHostEvent>(), events)
    assertEquals(1, host.destroyCalls)
    assertEquals(0, host.attachCalls)
  }

  @Test
  fun returnsNativeSurfaceAttachEvents() {
    val events: List<ExplorerHostEvent> = listOf(ExplorerHostEvent.SurfaceAttached(640, 480))
    val host = FakeExplorerHost(attachEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.attachSurface(fakeSurface(), 640, 480, 2f))
    assertEquals(1, host.attachCalls)
    assertEquals(SurfaceMetrics(640, 480, 2f), host.lastAttachedSurfaceMetrics)
    assertEquals(0, host.resizeCalls)
    assertEquals(0, host.detachCalls)
  }

  @Test
  fun returnsNativeSurfaceResizeEvents() {
    val events: List<ExplorerHostEvent> = listOf(ExplorerHostEvent.SurfaceResized(800, 600))
    val host = FakeExplorerHost(resizeEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.resizeSurface(800, 600, 2f))
    assertEquals(0, host.attachCalls)
    assertEquals(1, host.resizeCalls)
    assertEquals(SurfaceMetrics(800, 600, 2f), host.lastResizedSurfaceMetrics)
    assertEquals(0, host.detachCalls)
  }

  @Test
  fun returnsNativeSurfaceDetachEvents() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.SurfaceDetached)
    val host = FakeExplorerHost(detachEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.detachSurface())
    assertEquals(0, host.attachCalls)
    assertEquals(0, host.resizeCalls)
    assertEquals(1, host.detachCalls)
  }

  @Test
  fun rejectsNegativeSurfaceAttachDimensions() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertThrows(IllegalArgumentException::class.java) {
      binding.attachSurface(fakeSurface(), -1, 480, 2f)
    }
    assertEquals(0, host.attachCalls)
  }

  @Test
  fun rejectsNegativeSurfaceResizeDimensions() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertThrows(IllegalArgumentException::class.java) {
      binding.resizeSurface(800, -1, 2f)
    }
    assertEquals(0, host.resizeCalls)
  }

  @Test
  fun rejectsNonPositiveSurfaceDensity() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertThrows(IllegalArgumentException::class.java) {
      binding.attachSurface(fakeSurface(), 640, 480, 0f)
    }
    assertEquals(0, host.attachCalls)
  }

  @Test
  fun returnsEmptyListsWhenTheHostHasNoImmediateEvents() {
    val host = FakeExplorerHost(loadStatus = 0)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(emptyList(), binding.loadUrl("https://example.com"))
  }

  @Test
  fun drainsUpdateEventsFromPerformUpdates() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.LoadStatusChanged("Complete"))
    val host = FakeExplorerHost(updateEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.performUpdates())
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsTouchEventsToTheHost() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.UrlChanged("https://example.com/"))
    val host = FakeExplorerHost(touchEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.dispatchTouchEvent(TOUCH_EVENT_DOWN, 4, 12.5f, 30.25f))
    assertEquals(1, host.touchDispatchCalls)
    assertEquals(TouchDispatch(TOUCH_EVENT_DOWN, 4, 12.5f, 30.25f), host.lastTouchDispatch)
  }

  @Test
  fun forwardsContextMenuTriggersToTheHost() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.ContextMenuDismissed("context-menu-1"))
    val host = FakeExplorerHost(triggerContextMenuEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.triggerContextMenu(25f, 50f))
    assertEquals(ContextMenuTrigger(25f, 50f), host.lastContextMenuTrigger)
  }

  @Test
  fun forwardsReloadToTheHost() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.LoadStatusChanged("Started"))
    val host = FakeExplorerHost(reloadEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.reload())
    assertEquals(1, host.reloadCalls)
  }

  @Test
  fun forwardsGoBackToTheHost() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(emptyList(), binding.goBack())
    assertEquals(1, host.goBackCalls)
  }

  @Test
  fun forwardsGoForwardToTheHost() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(emptyList(), binding.goForward())
    assertEquals(1, host.goForwardCalls)
  }

  @Test
  fun forwardsFocusToTheHost() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.FocusChanged(true))
    val host = FakeExplorerHost(focusEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.focus())
    assertEquals(1, host.focusCalls)
  }

  @Test
  fun forwardsBlurToTheHost() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.FocusChanged(false))
    val host = FakeExplorerHost(blurEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.blur())
    assertEquals(1, host.blurCalls)
  }

  @Test
  fun forwardsMountedControllerCommandsThroughTheCurrentControllerHandle() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertEquals(0, binding.sendControllerCommand("{\"command\":\"reload\"}"))
    assertEquals(
      ControllerCommandEnvelope("controller-handle", "{\"command\":\"reload\"}"),
      host.lastControllerCommand
    )
  }

  @Test
  fun mountedControllerCommandsReturnNullPointerStatusWithoutAControllerHandle() {
    val host = FakeExplorerHost(controllerHandle = null)
    val binding = ExplorerViewBinding(host)

    assertEquals(2, binding.sendControllerCommand("{\"command\":\"reload\"}"))
    assertEquals(null, host.lastControllerCommand)
  }

  @Test
  fun forwardsSimpleDialogResolutionToTheHost() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.LoadStatusChanged("Complete"))
    val host = FakeExplorerHost(resolveSimpleDialogEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(
      events,
      binding.resolveSimpleDialog("dialog-1", SIMPLE_DIALOG_ACTION_CONFIRM, "servo")
    )
    assertEquals(SimpleDialogResolution("dialog-1", SIMPLE_DIALOG_ACTION_CONFIRM, "servo"), host.lastResolvedSimpleDialog)
  }

  @Test
  fun forwardsSelectElementResolutionThroughPerformUpdates() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.LoadStatusChanged("Complete"))
    val host = FakeExplorerHost(resolveSelectElementEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.resolveSelectElement("select-1", intArrayOf(4, 9)))
    assertEquals(SelectElementResolution("select-1", listOf(4, 9)), host.lastResolvedSelectElement)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsContextMenuResolutionThroughPerformUpdates() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.ContextMenuDismissed("context-menu-1"))
    val host = FakeExplorerHost(resolveContextMenuEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.resolveContextMenu("context-menu-1", "copy-link"))
    assertEquals(ContextMenuResolution("context-menu-1", "copy-link"), host.lastResolvedContextMenu)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsControllerContextMenuResolutionWithoutPerformUpdates() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

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
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.ContextMenuDismissed("context-menu-2"))
    val host = FakeExplorerHost(dismissContextMenuEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.dismissContextMenu("context-menu-2"))
    assertEquals("context-menu-2", host.lastDismissedContextMenuId)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsControllerContextMenuDismissalWithoutPerformUpdates() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

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
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.FilePickerDismissed("file-picker-1"))
    val host = FakeExplorerHost(resolveFilePickerEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.resolveFilePicker("file-picker-1", arrayOf("/cache/one.txt")))
    assertEquals(
      FilePickerResolution("file-picker-1", listOf("/cache/one.txt")),
      host.lastResolvedFilePicker
    )
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsFilePickerDismissalThroughPerformUpdates() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.FilePickerDismissed("file-picker-2"))
    val host = FakeExplorerHost(dismissFilePickerEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.dismissFilePicker("file-picker-2"))
    assertEquals("file-picker-2", host.lastDismissedFilePickerId)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun exposesAndResolvesPendingPermissionRequests() {
    val pendingRequest = ExplorerHostEvent.PermissionRequested("geolocation", "http://127.0.0.1:3000")
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.LoadStatusChanged("Complete"))
    val host = FakeExplorerHost(pendingPermissionRequest = pendingRequest, resolvePermissionEvents = events)
    val binding = ExplorerViewBinding(host)

    assertEquals(true, binding.hasPendingPermissionRequest())
    assertEquals(pendingRequest, binding.getPendingPermissionRequest())
    assertPlainEvents(events, binding.resolvePermission(true))
    assertEquals(true, host.lastResolvedPermissionAllow)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsNavigationRequestResolutionThroughPerformUpdates() {
    val events = listOf<ExplorerHostEvent>(ExplorerHostEvent.LoadStatusChanged("Started"))
    val host = FakeExplorerHost(resolveNavigationRequestEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.resolveNavigationRequest("navigation-1", false))
    assertEquals(NavigationRequestResolution("navigation-1", false), host.lastResolvedNavigationRequest)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsImeCompositionThroughPerformUpdates() {
    val events =
      listOf<ExplorerHostEvent>(
        ExplorerHostEvent.InputMethodRequested(
          inputMethodId = "ime-1",
          type = "text",
          text = "Servo",
          insertionPoint = 5,
          multiline = false,
          allowVirtualKeyboard = true
        )
      )
    val host = FakeExplorerHost(updateEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.dispatchImeComposition(IME_COMPOSITION_END, "Servo"))
    assertEquals(ImeDispatch(IME_COMPOSITION_END, "Servo"), host.lastImeDispatch)
    assertEquals(1, host.performUpdatesCalls)
  }

  @Test
  fun forwardsKeyboardKeysAndImeDismissalToTheHost() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(emptyList(), binding.dispatchKeyboardKey(KEYBOARD_KEY_BACKSPACE))
    assertPlainEvents(emptyList(), binding.dismissInputMethod())
    assertEquals(KEYBOARD_KEY_BACKSPACE, host.lastKeyboardKey)
    assertEquals(1, host.dismissInputMethodCalls)
  }

  @Test
  fun navigationCommandsReturnEmptyListAfterDispose() {
    val host = FakeExplorerHost()
    val binding = ExplorerViewBinding(host)

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
    val host = FakeExplorerHost(loadStatus = 0)
    val binding = ExplorerViewBinding(host)

    binding.dispose()
    binding.dispose()

    assertEquals(1, host.destroyCalls)
  }

  @Test
  fun loadUrlAfterDisposeReturnsASingleErrorEvent() {
    val host = FakeExplorerHost(loadStatus = 0)
    val binding = ExplorerViewBinding(host)

    binding.dispose()

    assertPlainEvents(
      listOf(ExplorerHostEvent.Error("https://example.com", 2, "binding has been disposed")),
      binding.loadUrl("https://example.com")
    )
  }

  @Test
  fun preservesServoStyleDelegatePayloads() {
    val events =
      listOf<ExplorerHostEvent>(
        ExplorerHostEvent.PageTitleChanged("Example Domain"),
        ExplorerHostEvent.StatusTextChanged("Done"),
        ExplorerHostEvent.HistoryChanged(
          entries = listOf("https://example.com/", "https://www.rust-lang.org/"),
          current = 1,
          canGoBack = true,
          canGoForward = false
        ),
        ExplorerHostEvent.Crashed(
          url = "https://example.com/",
          reason = "renderer panic",
          backtrace = "stack"
        ),
        ExplorerHostEvent.SimpleDialogRequested(
          dialogId = "dialog-1",
          kind = "prompt",
          message = "Name?",
          defaultValue = "Servo"
        ),
        ExplorerHostEvent.SimpleDialogDismissed("dialog-1"),
        ExplorerHostEvent.InputMethodRequested(
          inputMethodId = "ime-1",
          type = "email",
          text = "servo@example.com",
          insertionPoint = 17,
          multiline = false,
          allowVirtualKeyboard = true
        ),
        ExplorerHostEvent.InputMethodDismissed("ime-1"),
        ExplorerHostEvent.ContextMenuRequested(
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
        ExplorerHostEvent.ContextMenuDismissed("context-menu-1"),
        ExplorerHostEvent.FilePickerRequested(
          filePickerId = "file-picker-1",
          currentPaths = listOf("/cache/one.txt"),
          filterPatterns = listOf("txt", "png"),
          allowSelectMultiple = true
        ),
        ExplorerHostEvent.FilePickerDismissed("file-picker-1"),
        ExplorerHostEvent.PermissionRequested("geolocation", "http://127.0.0.1:3000"),
        ExplorerHostEvent.FocusChanged(true),
        ExplorerHostEvent.CursorChanged("pointer"),
        ExplorerHostEvent.FullscreenChanged(true),
        ExplorerHostEvent.Closed
      )
    val host = FakeExplorerHost(updateEvents = events)
    val binding = ExplorerViewBinding(host)

    assertPlainEvents(events, binding.performUpdates())
  }
}

private fun assertPlainEvents(expected: List<ExplorerHostEvent>, actual: List<ExplorerHostEvent>) {
  assertEquals(expected, actual)
  assertEquals(expected.javaClass, actual.javaClass)
}

private fun createSurfaceCoordinator(
  binding: ExplorerViewBinding,
  events: MutableList<ExplorerHostEvent>
) = ExplorerSurfaceLifecycleCoordinator(
  onSurfaceAttached = { surface, width, height ->
    events += binding.attachSurface(surface, width, height, 2f)
  },
  onSurfaceResized = { width, height -> events += binding.resizeSurface(width, height, 2f) },
  onSurfaceDetached = { events += binding.detachSurface() },
  onLoadUrl = { events += binding.loadUrl(it) },
  onRenderingStarted = {},
  onRenderingStopped = {}
)

private fun fakeSurface(): Surface {
  val unsafeClass = Class.forName("sun.misc.Unsafe")
  val field = unsafeClass.getDeclaredField("theUnsafe")
  field.isAccessible = true
  val unsafe = field.get(null)
  val allocateInstance = unsafeClass.getMethod("allocateInstance", Class::class.java)
  return allocateInstance.invoke(unsafe, Surface::class.java) as Surface
}

private class FakeExplorerHost(
  private val controllerHandle: String? = "controller-handle",
  private val loadStatus: Int = 0,
  private val loadEvents: List<ExplorerHostEvent> = emptyList(),
  private val attachEvents: List<ExplorerHostEvent> = emptyList(),
  private val resizeEvents: List<ExplorerHostEvent> = emptyList(),
  private val detachEvents: List<ExplorerHostEvent> = emptyList(),
  private val touchEvents: List<ExplorerHostEvent> = emptyList(),
  private val updateEvents: List<ExplorerHostEvent> = emptyList(),
  private val reloadEvents: List<ExplorerHostEvent> = emptyList(),
  private val goBackEvents: List<ExplorerHostEvent> = emptyList(),
  private val goForwardEvents: List<ExplorerHostEvent> = emptyList(),
  private val focusEvents: List<ExplorerHostEvent> = emptyList(),
  private val blurEvents: List<ExplorerHostEvent> = emptyList(),
  private val resolveSimpleDialogEvents: List<ExplorerHostEvent> = emptyList(),
  private val resolveSelectElementEvents: List<ExplorerHostEvent> = emptyList(),
  private val resolveContextMenuEvents: List<ExplorerHostEvent> = emptyList(),
  private val dismissContextMenuEvents: List<ExplorerHostEvent> = emptyList(),
  private val resolveFilePickerEvents: List<ExplorerHostEvent> = emptyList(),
  private val dismissFilePickerEvents: List<ExplorerHostEvent> = emptyList(),
  private val triggerContextMenuEvents: List<ExplorerHostEvent> = emptyList(),
  private val pendingPermissionRequest: ExplorerHostEvent.PermissionRequested? = null,
  private val resolvePermissionEvents: List<ExplorerHostEvent> = emptyList(),
  private val resolveNavigationRequestEvents: List<ExplorerHostEvent> = emptyList()
) : ExplorerHost {
  val loadedUrls = mutableListOf<String>()
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
  private var pendingEvents: List<ExplorerHostEvent> = emptyList()

  override fun controllerHandle(): String? = controllerHandle

  override fun loadUrl(input: String): Int {
    loadedUrls += input
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

  override fun getPendingPermissionRequest(): ExplorerHostEvent.PermissionRequested? = pendingPermissionRequest

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

  override fun drainEvents(): List<ExplorerHostEvent> =
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
