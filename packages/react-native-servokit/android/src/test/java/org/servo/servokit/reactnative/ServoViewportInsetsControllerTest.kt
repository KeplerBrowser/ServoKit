package org.servo.servokit.reactnative

import org.junit.Assert.assertEquals
import org.junit.Test

class ServoViewportInsetsControllerTest {
  @Test
  fun returnsFullViewportWhenSurfaceAttachesWithoutIme() {
    val controller = ServoViewportInsetsController()

    assertEquals(ServoViewportSize(1080, 1920), controller.onSurfaceAttached(1080, 1920))
  }

  @Test
  fun shrinksViewportByImeInset() {
    val controller = ServoViewportInsetsController()
    controller.onSurfaceAttached(1080, 1920)

    assertEquals(ServoViewportSize(1080, 1320), controller.onImeInsetChanged(600))
  }

  @Test
  fun restoresViewportWhenImeDismisses() {
    val controller = ServoViewportInsetsController()
    controller.onSurfaceAttached(1080, 1920)
    controller.onImeInsetChanged(600)

    assertEquals(ServoViewportSize(1080, 1920), controller.onImeInsetChanged(0))
  }

  @Test
  fun suppressesDuplicateViewportUpdates() {
    val controller = ServoViewportInsetsController()

    assertEquals(ServoViewportSize(1080, 1920), controller.onSurfaceAttached(1080, 1920))
    assertEquals(null, controller.onImeInsetChanged(0))
    assertEquals(null, controller.onSurfaceResized(1080, 1920))
  }

  @Test
  fun reappliesImeInsetToLaterSurfaceResize() {
    val controller = ServoViewportInsetsController()
    controller.onSurfaceAttached(1080, 1920)
    controller.onImeInsetChanged(600)

    assertEquals(ServoViewportSize(1080, 1520), controller.onSurfaceResized(1080, 2120))
  }

  @Test
  fun avoidsDoubleShrinkingWhenLayoutAlreadyResizedForIme() {
    val controller = ServoViewportInsetsController()
    controller.onSurfaceAttached(1080, 1920)
    controller.onImeInsetChanged(600)

    assertEquals(null, controller.onSurfaceResized(1080, 1320))
  }

  @Test
  fun clampsViewportHeightToAtLeastOnePixel() {
    val controller = ServoViewportInsetsController()
    controller.onSurfaceAttached(1080, 400)

    assertEquals(ServoViewportSize(1080, 1), controller.onImeInsetChanged(800))
  }

  @Test
  fun clearsImeInsetOnSurfaceDetachForOrientationChange() {
    val controller = ServoViewportInsetsController()
    controller.onSurfaceAttached(1080, 1920)
    controller.onImeInsetChanged(600) // portrait keyboard

    controller.onSurfaceDetached()

    // After detach, reattach with landscape dimensions
    // Should use full landscape height, not portrait inset
    assertEquals(ServoViewportSize(1920, 1080), controller.onSurfaceAttached(1920, 1080))
  }
}
