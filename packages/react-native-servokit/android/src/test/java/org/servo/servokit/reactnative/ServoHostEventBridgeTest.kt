package org.servo.servokit.reactnative

import org.servo.servokit.androidhost.*

import org.junit.Assert.assertEquals
import org.junit.Test

class ServoHostEventBridgeTest {
  @Test
  fun decodesScalarHostEvents() {
    val cases =
      listOf(
        """{"name":"navigationRequested","payload":{"navigationId":"navigation-1","url":"https://example.com/"}}""" to
          ServoHostEvent.NavigationRequested("navigation-1", "https://example.com/"),
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
        """{"name":"error","payload":{"url":"https:///","code":1,"message":"invalid url"}}""" to
          ServoHostEvent.Error("https:///", 1, "invalid url"),
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
  fun decodesNullablePayloadFields() {
    val cases =
      listOf(
        """{"name":"pageTitleChanged","payload":{"title":null}}""" to
          ServoHostEvent.PageTitleChanged(null),
        """{"name":"statusTextChanged","payload":{"status":null}}""" to
          ServoHostEvent.StatusTextChanged(null),
        """{"name":"crashed","payload":{"url":"https://example.com/","reason":"boom","backtrace":null}}""" to
          ServoHostEvent.Crashed("https://example.com/", "boom", null),
        """{"name":"error","payload":{"url":null,"code":2,"message":"no current entry"}}""" to
          ServoHostEvent.Error(null, 2, "no current entry"),
        """{"name":"simpleDialogRequested","payload":{"dialogId":"dialog-1","kind":"prompt","message":"Name?","defaultValue":null}}""" to
          ServoHostEvent.SimpleDialogRequested("dialog-1", "prompt", "Name?", null),
        """{"name":"inputMethodRequested","payload":{"inputMethodId":"ime-1","type":"text","text":"hello","insertionPoint":null,"multiline":false,"allowVirtualKeyboard":true}}""" to
          ServoHostEvent.InputMethodRequested(
            inputMethodId = "ime-1",
            type = "text",
            text = "hello",
            insertionPoint = null,
            multiline = false,
            allowVirtualKeyboard = true
          )
      )

    cases.forEach { (json, expected) ->
      assertEquals(expected, ServoHostEventBridge.decode(json))
    }
  }

  @Test
  fun decodesNestedBridgePayloads() {
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
                  SelectElementOption(id = 4, label = "Layout", isDisabled = true)
                )
            )
          ),
        selectedOptions = listOf(1, 4),
        allowSelectMultiple = true
      ),
      ServoHostEventBridge.decode(
        """{"name":"selectElementRequested","payload":{"selectElementId":"select-1","options":[{"type":"option","id":1,"label":"Servo","isDisabled":false},{"type":"optgroup","label":"Engines","options":[{"id":4,"label":"Layout","isDisabled":true}]}],"selectedOptions":[1,4],"allowSelectMultiple":true}}"""
      )
    )

    assertEquals(
      ServoHostEvent.ContextMenuRequested(
        contextMenuId = "context-menu-1",
        x = 12,
        y = 24,
        width = 32,
        height = 48,
        elementInfo =
          ContextMenuElementInformation(
            isLink = true,
            isImage = false,
            isEditableText = false,
            hasSelection = false,
            linkUrl = "https://example.com/",
            imageUrl = null,
            contextType = "link"
          ),
        items =
          listOf(
            ContextMenuItem.Item(
              label = "Copy link",
              action = "copy-link",
              enabled = true
            ),
            ContextMenuItem.Separator
          )
      ),
      ServoHostEventBridge.decode(
        """{"name":"contextMenuRequested","payload":{"contextMenuId":"context-menu-1","x":12,"y":24,"width":32,"height":48,"elementInfo":{"isLink":true,"isImage":false,"isEditableText":false,"hasSelection":false,"linkUrl":"https://example.com/","imageUrl":null,"contextType":"link"},"items":[{"type":"item","label":"Copy link","action":"copy-link","enabled":true},{"type":"separator"}]}}"""
      )
    )

    assertEquals(
      ServoHostEvent.FilePickerRequested(
        filePickerId = "file-picker-1",
        currentPaths = listOf("/cache/one.txt"),
        filterPatterns = listOf("txt", "png"),
        allowSelectMultiple = true
      ),
      ServoHostEventBridge.decode(
        """{"name":"filePickerRequested","payload":{"filePickerId":"file-picker-1","currentPaths":["/cache/one.txt"],"filterPatterns":["txt","png"],"allowSelectMultiple":true}}"""
      )
    )
  }
}
