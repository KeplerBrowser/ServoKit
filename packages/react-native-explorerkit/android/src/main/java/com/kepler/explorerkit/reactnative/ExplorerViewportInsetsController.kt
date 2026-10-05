package com.kepler.explorerkit.reactnative

internal data class ExplorerViewportSize(val width: Int, val height: Int)

internal class ExplorerViewportInsetsController {
  private var surfaceWidth = 0
  private var surfaceHeight = 0
  private var unobscuredSurfaceHeight = 0
  private var imeBottomInset = 0
  private var appliedViewport: ExplorerViewportSize? = null

  fun onSurfaceAttached(width: Int, height: Int): ExplorerViewportSize? {
    surfaceWidth = width
    surfaceHeight = height
    return nextViewport()
  }

  fun onSurfaceResized(width: Int, height: Int): ExplorerViewportSize? {
    surfaceWidth = width
    surfaceHeight = height
    return nextViewport()
  }

  fun onImeInsetChanged(bottomInset: Int): ExplorerViewportSize? {
    imeBottomInset = bottomInset.coerceAtLeast(0)
    return nextViewport()
  }

  fun onSurfaceDetached() {
    appliedViewport = null
    surfaceWidth = 0
    surfaceHeight = 0
    unobscuredSurfaceHeight = 0
    imeBottomInset = 0
  }

  private fun nextViewport(): ExplorerViewportSize? {
    if (surfaceWidth <= 0 || surfaceHeight <= 0) {
      return null
    }

    if (imeBottomInset == 0) {
      unobscuredSurfaceHeight = surfaceHeight
    }

    val alreadyAppliedInset =
      (unobscuredSurfaceHeight - surfaceHeight)
        .coerceAtLeast(0)
    val remainingImeInset =
      (imeBottomInset - alreadyAppliedInset)
        .coerceAtLeast(0)

    val viewport =
      ExplorerViewportSize(
        width = surfaceWidth,
        height = (surfaceHeight - remainingImeInset).coerceAtLeast(1)
      )
    if (viewport == appliedViewport) {
      return null
    }

    appliedViewport = viewport
    return viewport
  }
}
