package com.kepler.explorerkit.androidhost

import android.view.Surface
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class ExplorerSurfaceLifecycleCoordinatorTest {
  @Test
  fun forwardsSurfaceAttachWithProvidedWidthAndHeight() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun startsRenderingWhenTheSurfaceAttaches() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun ignoresZeroSizedSurfaceCreate() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 0, 0)

    assertEquals(emptyList<SurfaceLifecycleCall>(), calls)
  }

  @Test
  fun firstUsableResizeAttachesAfterAZeroSizedCreate() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 0, 0)
    coordinator.onSurfaceResized(800, 600)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Attach(800, 600),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun forwardsSurfaceResizeWithProvidedWidthAndHeight() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onSurfaceResized(800, 600)

    assertEquals(listOf(SurfaceLifecycleCall.Resize(800, 600)), calls)
  }

  @Test
  fun forwardsSurfaceDetach() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onSurfaceDestroyed()

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering
      ),
      calls
    )
  }

  @Test
  fun detachesWhenTheViewIsDisposedBeforeTheSurfaceIsDestroyed() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onViewDisposed()

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering
      ),
      calls
    )
  }

  @Test
  fun waitsForAUsableSizeBeforeForwardingTheInitialAttach() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 0, 0)
    coordinator.onSurfaceResized(800, 600)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Attach(800, 600),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun disposingAfterSurfaceDestroyDoesNotDetachTwice() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onSurfaceDestroyed()
    coordinator.onViewDisposed()

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering
      ),
      calls
    )
  }

  @Test
  fun forwardsInitialNavigationBeforeTheSurfaceAttaches() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.loadUrl("https://example.com/")
    assertEquals(listOf(SurfaceLifecycleCall.LoadUrl("https://example.com/")), calls)
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.LoadUrl("https://example.com/"),
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun forwardsEveryInitialRequestBeforeSurfaceBinding() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.loadUrl("https://example.com/first")
    coordinator.loadUrl("https://example.com/latest")
    coordinator.loadUrl("https:///")
    val requests = listOf(
      SurfaceLifecycleCall.LoadUrl("https://example.com/first"),
      SurfaceLifecycleCall.LoadUrl("https://example.com/latest"),
      SurfaceLifecycleCall.LoadUrl("https:///")
    )
    assertEquals(requests, calls)
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      requests + listOf(
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun loadUrlAfterAttachNavigatesImmediately() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.loadUrl("https://example.com/")

    assertEquals(listOf(SurfaceLifecycleCall.LoadUrl("https://example.com/")), calls)
  }

  @Test
  fun keepsForwardingAcrossZeroSizedSurfaceRecreationBeforeFirstBind() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.loadUrl("https://example.com/")
    coordinator.onSurfaceCreated(fakeSurface(), 0, 0)
    coordinator.loadUrl("https://example.com/zero-sized")
    coordinator.onSurfaceDestroyed()
    coordinator.loadUrl("https:///")
    val requests = listOf(
      SurfaceLifecycleCall.LoadUrl("https://example.com/"),
      SurfaceLifecycleCall.LoadUrl("https://example.com/zero-sized"),
      SurfaceLifecycleCall.LoadUrl("https:///")
    )
    assertEquals(requests, calls)
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      requests + listOf(
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun keepsPostDetachNavigationUntilTheSurfaceRebinds() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onSurfaceDestroyed()
    coordinator.loadUrl("https://example.com/")
    coordinator.loadUrl("https:///")
    assertEquals(
      listOf(SurfaceLifecycleCall.Detach, SurfaceLifecycleCall.StopRendering),
      calls
    )
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering,
        SurfaceLifecycleCall.LoadUrl("https:///"),
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun doesNotReplayConsumedInitialNavigationAfterASurfaceRebind() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.loadUrl("https://example.com/")
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onSurfaceDestroyed()
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering,
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun keepsNavigationSubmittedInsideFirstAttachBufferedUntilRebind() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    lateinit var coordinator: ExplorerSurfaceLifecycleCoordinator
    var attachCalls = 0
    coordinator = createCoordinator(calls) {
      if (++attachCalls == 1) {
        coordinator.loadUrl("https://example.com/reentrant")
        assertEquals(listOf(SurfaceLifecycleCall.Attach(640, 480)), calls)
      }
    }

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    assertEquals(
      listOf(SurfaceLifecycleCall.Attach(640, 480), SurfaceLifecycleCall.StartRendering),
      calls
    )
    calls.clear()
    coordinator.onSurfaceDestroyed()
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering,
        SurfaceLifecycleCall.LoadUrl("https://example.com/reentrant"),
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun keepsNavigationBufferedAfterFirstAttachThrows() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    var attachCalls = 0
    val coordinator = createCoordinator(calls) {
      if (++attachCalls == 1) {
        throw IllegalStateException("attach failed")
      }
    }

    assertThrows(IllegalStateException::class.java) {
      coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    }
    calls.clear()
    coordinator.loadUrl("https://example.com/retry")
    assertEquals(emptyList<SurfaceLifecycleCall>(), calls)
    coordinator.onSurfaceDestroyed()
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.LoadUrl("https://example.com/retry"),
        SurfaceLifecycleCall.Attach(640, 480),
        SurfaceLifecycleCall.StartRendering
      ),
      calls
    )
  }

  @Test
  fun stopsInitialForwardingWhenDisposedBeforeAnyAttach() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)
    coordinator.loadUrl("https://example.com/")
    calls.clear()

    coordinator.onViewDisposed()
    coordinator.loadUrl("https://example.com/after-dispose")

    assertEquals(emptyList<SurfaceLifecycleCall>(), calls)
  }

  private fun createCoordinator(
    calls: MutableList<SurfaceLifecycleCall>,
    onAttach: () -> Unit = {}
  ) = ExplorerSurfaceLifecycleCoordinator(
    onSurfaceAttached = { _, width, height ->
      calls += SurfaceLifecycleCall.Attach(width, height)
      onAttach()
    },
    onSurfaceResized = { width, height -> calls += SurfaceLifecycleCall.Resize(width, height) },
    onSurfaceDetached = { calls += SurfaceLifecycleCall.Detach },
    onLoadUrl = { url -> calls += SurfaceLifecycleCall.LoadUrl(url) },
    onRenderingStarted = { calls += SurfaceLifecycleCall.StartRendering },
    onRenderingStopped = { calls += SurfaceLifecycleCall.StopRendering }
  )
}

private sealed interface SurfaceLifecycleCall {
  data class Attach(val width: Int, val height: Int) : SurfaceLifecycleCall

  data class Resize(val width: Int, val height: Int) : SurfaceLifecycleCall

  data object Detach : SurfaceLifecycleCall

  data class LoadUrl(val url: String) : SurfaceLifecycleCall

  data object StartRendering : SurfaceLifecycleCall

  data object StopRendering : SurfaceLifecycleCall
}

private fun fakeSurface(): Surface {
  val unsafeClass = Class.forName("sun.misc.Unsafe")
  val field = unsafeClass.getDeclaredField("theUnsafe")
  field.isAccessible = true
  val unsafe = field.get(null)
  val allocateInstance = unsafeClass.getMethod("allocateInstance", Class::class.java)
  return allocateInstance.invoke(unsafe, Surface::class.java) as Surface
}
