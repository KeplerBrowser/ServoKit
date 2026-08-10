package org.servo.servokit.androidhost

import org.junit.Assert.assertEquals
import org.junit.Test

class ServoHostEventBridgeTest {
  @Test
  fun decodesScalarHostEvents() {
    val cases =
      listOf(
        """{"name":"navigationRequested","payload":{"navigationId":"navigation-1","url":"https://example.com/"}}""" to
          ServoHostEvent.NavigationRequested("navigation-1", "https://example.com/"),
        """{"name":"popupRequested","payload":{"parentWebViewId":"parent-1","parentUrl":"https://parent.test/","targetUrl":null,"windowFeatures":null,"policy":"default-deny"}}""" to
          ServoHostEvent.PopupRequested(
            parentWebViewId = "parent-1",
            parentUrl = "https://parent.test/",
            targetUrl = null,
            windowFeatures = null,
            policy = "default-deny"
          ),
        """{"name":"popupCreated","payload":{"parentWebViewId":"parent-1","childWebViewId":"child-1","parentUrl":"https://parent.test/","targetUrl":null,"windowFeatures":null,"policy":"managed-child"}}""" to
          ServoHostEvent.PopupCreated(
            parentWebViewId = "parent-1",
            childWebViewId = "child-1",
            parentUrl = "https://parent.test/",
            targetUrl = null,
            windowFeatures = null,
            policy = "managed-child"
          ),
        """{"name":"urlChanged","payload":{"url":"https://example.com/"}}""" to
          ServoHostEvent.UrlChanged("https://example.com/"),
        """{"name":"pageTitleChanged","payload":{"title":"Example Domain"}}""" to
          ServoHostEvent.PageTitleChanged("Example Domain"),
        """{"name":"statusTextChanged","payload":{"status":""}}""" to
          ServoHostEvent.StatusTextChanged(""),
        """{"name":"loadStatusChanged","payload":{"status":"Complete"}}""" to
          ServoHostEvent.LoadStatusChanged("Complete"),
        """{"name":"historyChanged","payload":{"entries":["https://a.example/","https://b.example/"],"current":1,"canGoBack":true,"canGoForward":false}}""" to
          ServoHostEvent.HistoryChanged(
            entries = listOf("https://a.example/", "https://b.example/"),
            current = 1,
            canGoBack = true,
            canGoForward = false
          ),
        """{"name":"closed","payload":{}}""" to ServoHostEvent.Closed,
        """{"name":"crashed","payload":{"url":null,"reason":"boom","backtrace":null}}""" to
          ServoHostEvent.Crashed(null, "boom", null),
        """{"name":"error","payload":{"url":"https:///","code":1,"message":"invalid url"}}""" to
          ServoHostEvent.Error("https:///", 1, "invalid url"),
        """{"name":"error","payload":{"url":null,"code":2,"message":"no current entry"}}""" to
          ServoHostEvent.Error(null, 2, "no current entry"),
        """{"name":"javascriptEvaluationResult","payload":{"evaluationId":"evaluation-1","ok":true,"valueJson":"{\"type\":\"string\",\"value\":\"Servo\"}","errorType":null}}""" to
          ServoHostEvent.JavaScriptEvaluationResult(
            evaluationId = "evaluation-1",
            ok = true,
            valueJson = "{\"type\":\"string\",\"value\":\"Servo\"}",
            errorType = null
          ),
        """{"name":"simpleDialogDismissed","payload":{"dialogId":"dialog-1"}}""" to
          ServoHostEvent.SimpleDialogDismissed("dialog-1"),
        """{"name":"inputMethodDismissed","payload":{"inputMethodId":"ime-1"}}""" to
          ServoHostEvent.InputMethodDismissed("ime-1"),
        """{"name":"selectElementDismissed","payload":{"selectElementId":"select-1"}}""" to
          ServoHostEvent.SelectElementDismissed("select-1"),
        """{"name":"contextMenuDismissed","payload":{"contextMenuId":"context-menu-1"}}""" to
          ServoHostEvent.ContextMenuDismissed("context-menu-1"),
        """{"name":"filePickerDismissed","payload":{"filePickerId":"file-picker-1"}}""" to
          ServoHostEvent.FilePickerDismissed("file-picker-1"),
        """{"name":"permissionRequested","payload":{"permission":"geolocation","origin":"http://127.0.0.1:3000"}}""" to
          ServoHostEvent.PermissionRequested("geolocation", "http://127.0.0.1:3000"),
        """{"name":"focusChanged","payload":{"isFocused":true}}""" to
          ServoHostEvent.FocusChanged(true),
        """{"name":"cursorChanged","payload":{"cursor":"pointer"}}""" to
          ServoHostEvent.CursorChanged("pointer"),
        """{"name":"fullscreenChanged","payload":{"isFullscreen":true}}""" to
          ServoHostEvent.FullscreenChanged(true),
        """{"name":"surfaceAttached","payload":{"width":640,"height":480}}""" to
          ServoHostEvent.SurfaceAttached(640, 480),
        """{"name":"surfaceResized","payload":{"width":800,"height":600}}""" to
          ServoHostEvent.SurfaceResized(800, 600),
        """{"name":"surfaceDetached","payload":{}}""" to ServoHostEvent.SurfaceDetached
      )

    cases.forEach { (json, expected) ->
      assertEquals(expected, ServoHostEventBridge.decode(json))
    }
  }

  @Test
  fun decodesSelectElementRequestWithMultipleSelectedOptions() {
    val json =
      """{"name":"selectElementRequested","payload":{"selectElementId":"select-1","options":[{"type":"option","id":1,"label":"Servo","isDisabled":false},{"type":"optgroup","label":"Engines","options":[{"id":4,"label":"Layout","isDisabled":false},{"id":9,"label":"GPU","isDisabled":true}]}],"selectedOptions":[1,4],"allowSelectMultiple":true}}"""

    assertEquals(
      ServoHostEvent.SelectElementRequested(
        selectElementId = "select-1",
        options =
          listOf(
            SelectElementOptionOrOptgroup.Option(
              SelectElementOption(id = 1, label = "Servo", isDisabled = false)
            ),
            SelectElementOptionOrOptgroup.Optgroup(
              label = "Engines",
              options =
                listOf(
                  SelectElementOption(id = 4, label = "Layout", isDisabled = false),
                  SelectElementOption(id = 9, label = "GPU", isDisabled = true)
                )
            )
          ),
        selectedOptions = listOf(1, 4),
        allowSelectMultiple = true
      ),
      ServoHostEventBridge.decode(json)
    )
  }
}
