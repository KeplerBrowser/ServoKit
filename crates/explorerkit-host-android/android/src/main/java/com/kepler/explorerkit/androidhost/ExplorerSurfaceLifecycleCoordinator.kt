package com.kepler.explorerkit.androidhost

import android.view.Surface

/**
 * Proof-only Surface lifecycle coordinator shared by Android host adapters.
 *
 * Initial navigation reaches Rust immediately for validation and retention, until the first usable
 * surface attach attempt or disposal. Later detached navigation is retained until reattachment,
 * preserving the existing behavior after failed surface transitions. The app or framework owns the
 * actual `SurfaceView`, while Rust remains the browser lifecycle and engine owner.
 */
class ExplorerSurfaceLifecycleCoordinator(
  onSurfaceAttached: (Surface, Int, Int) -> Unit,
  onSurfaceResized: (Int, Int) -> Unit,
  onSurfaceDetached: () -> Unit,
  onLoadUrl: (String) -> Unit,
  onRenderingStarted: () -> Unit,
  onRenderingStopped: () -> Unit
) {
  private val attachSurface = onSurfaceAttached
  private val resizeSurface = onSurfaceResized
  private val detachSurface = onSurfaceDetached
  private val requestNavigation = onLoadUrl
  private val startRendering = onRenderingStarted
  private val stopRendering = onRenderingStopped

  private var isSurfaceAvailable = false
  private var isSurfaceAttached = false
  private var isInitialNavigationPhase = true
  private var pendingNavigationUrl: String? = null
  private var surface: Surface? = null

  fun loadUrl(url: String) {
    if (isInitialNavigationPhase) {
      requestNavigation(url)
      return
    }

    pendingNavigationUrl = url
    if (isSurfaceAttached) {
      flushPendingNavigation()
    }
  }

  fun onSurfaceCreated(surface: Surface, width: Int, height: Int) {
    isSurfaceAvailable = true
    this.surface = surface
    attachSurfaceIfReady(width, height)
  }

  fun onSurfaceResized(width: Int, height: Int) {
    if (!isSurfaceAvailable) {
      return
    }

    if (!isSurfaceAttached) {
      attachSurfaceIfReady(width, height)
      return
    }

    resizeSurface(width, height)
  }

  fun onSurfaceDestroyed() {
    isSurfaceAvailable = false
    surface = null
    if (!isSurfaceAttached) {
      return
    }

    isSurfaceAttached = false
    detachSurface()
    stopRendering()
  }

  fun onViewDisposed() {
    isInitialNavigationPhase = false
    pendingNavigationUrl = null
    onSurfaceDestroyed()
  }

  private fun attachSurfaceIfReady(width: Int, height: Int) {
    val activeSurface = surface
    if (isSurfaceAttached || activeSurface == null || width <= 0 || height <= 0) {
      return
    }

    isInitialNavigationPhase = false
    flushPendingNavigation()
    attachSurface(activeSurface, width, height)
    isSurfaceAttached = true
    startRendering()
  }

  private fun flushPendingNavigation() {
    val url = pendingNavigationUrl ?: return
    pendingNavigationUrl = null
    requestNavigation(url)
  }
}
