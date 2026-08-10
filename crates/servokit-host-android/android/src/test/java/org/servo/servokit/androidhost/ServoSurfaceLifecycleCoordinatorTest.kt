package org.servo.servokit.androidhost

import android.view.Surface
import org.junit.Assert.assertEquals
import org.junit.Test

class ServoSurfaceLifecycleCoordinatorTest {
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
  fun flushesTheLatestInitialNavigationAfterLateSurfaceBinding() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.loadUrl("https://example.com/first")
    coordinator.loadUrl("https://example.com/latest")
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.LoadUrl("https://example.com/latest"),
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
  fun keepsPendingNavigationAcrossSurfaceRecreationBeforeFirstBind() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.loadUrl("https://example.com/")
    coordinator.onSurfaceCreated(fakeSurface(), 0, 0)
    coordinator.onSurfaceDestroyed()
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
  fun keepsPostDetachNavigationUntilTheSurfaceRebinds() {
    val calls = mutableListOf<SurfaceLifecycleCall>()
    val coordinator = createCoordinator(calls)

    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)
    calls.clear()

    coordinator.onSurfaceDestroyed()
    coordinator.loadUrl("https://example.com/")
    coordinator.onSurfaceCreated(fakeSurface(), 640, 480)

    assertEquals(
      listOf(
        SurfaceLifecycleCall.Detach,
        SurfaceLifecycleCall.StopRendering,
        SurfaceLifecycleCall.LoadUrl("https://example.com/"),
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

  private fun createCoordinator(
    calls: MutableList<SurfaceLifecycleCall>
  ) = ServoSurfaceLifecycleCoordinator(
    onSurfaceAttached = { _, width, height -> calls += SurfaceLifecycleCall.Attach(width, height) },
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
