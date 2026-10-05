package com.kepler.explorerkit.reactnative

import com.kepler.explorerkit.androidhost.*

import org.junit.Assert.assertEquals
import org.junit.Test

class ExplorerHostEventBridgeTest {
  @Test
  fun decodesScalarHostEvents() {
    val cases =
      listOf(
        """{"name":"navigationRequested","payload":{"navigationId":"navigation-1","url":"https://example.com/"}}""" to
          ExplorerHostEvent.NavigationRequested("navigation-1", "https://example.com/"),
        """{"name":"urlChanged","payload":{"url":"https://example.com/"}}""" to
          ExplorerHostEvent.UrlChanged("https://example.com/"),
        """{"name":"pageTitleChanged","payload":{"title":"Example Domain"}}""" to
          ExplorerHostEvent.PageTitleChanged("Example Domain"),
        """{"name":"statusTextChanged","payload":{"status":""}}""" to
          ExplorerHostEvent.StatusTextChanged(""),
        """{"name":"loadStatusChanged","payload":{"status":"Complete"}}""" to
          ExplorerHostEvent.LoadStatusChanged("Complete"),
        """{"name":"historyChanged","payload":{"entries":["https://a.example/","https://b.example/"],"current":1,"canGoBack":true,"canGoForward":false}}""" to
          ExplorerHostEvent.HistoryChanged(
            entries = listOf("https://a.example/", "https://b.example/"),
            current = 1,
            canGoBack = true,
            canGoForward = false
          ),
        """{"name":"closed","payload":{}}""" to ExplorerHostEvent.Closed,
        """{"name":"error","payload":{"url":"https:///","code":1,"message":"invalid url"}}""" to
          ExplorerHostEvent.Error("https:///", 1, "invalid url"),
        """{"name":"javascriptEvaluationResult","payload":{"evaluationId":"evaluation-1","ok":true,"valueJson":"{\"type\":\"string\",\"value\":\"Servo\"}","errorType":null}}""" to
          ExplorerHostEvent.JavaScriptEvaluationResult(
            evaluationId = "evaluation-1",
            ok = true,
            valueJson = "{\"type\":\"string\",\"value\":\"Servo\"}",
            errorType = null
          ),
        """{"name":"simpleDialogDismissed","payload":{"dialogId":"dialog-1"}}""" to
          ExplorerHostEvent.SimpleDialogDismissed("dialog-1"),
        """{"name":"inputMethodDismissed","payload":{"inputMethodId":"ime-1"}}""" to
          ExplorerHostEvent.InputMethodDismissed("ime-1"),
        """{"name":"selectElementDismissed","payload":{"selectElementId":"select-1"}}""" to
          ExplorerHostEvent.SelectElementDismissed("select-1"),
        """{"name":"contextMenuDismissed","payload":{"contextMenuId":"context-menu-1"}}""" to
          ExplorerHostEvent.ContextMenuDismissed("context-menu-1"),
        """{"name":"filePickerDismissed","payload":{"filePickerId":"file-picker-1"}}""" to
          ExplorerHostEvent.FilePickerDismissed("file-picker-1"),
        """{"name":"permissionRequested","payload":{"permission":"geolocation","origin":"http://127.0.0.1:3000"}}""" to
          ExplorerHostEvent.PermissionRequested("geolocation", "http://127.0.0.1:3000"),
        """{"name":"focusChanged","payload":{"isFocused":true}}""" to
          ExplorerHostEvent.FocusChanged(true),
        """{"name":"cursorChanged","payload":{"cursor":"pointer"}}""" to
          ExplorerHostEvent.CursorChanged("pointer"),
        """{"name":"fullscreenChanged","payload":{"isFullscreen":true}}""" to
          ExplorerHostEvent.FullscreenChanged(true),
        """{"name":"surfaceAttached","payload":{"width":640,"height":480}}""" to
          ExplorerHostEvent.SurfaceAttached(640, 480),
        """{"name":"surfaceResized","payload":{"width":800,"height":600}}""" to
          ExplorerHostEvent.SurfaceResized(800, 600),
        """{"name":"surfaceDetached","payload":{}}""" to ExplorerHostEvent.SurfaceDetached
      )

    cases.forEach { (json, expected) ->
      assertEquals(expected, ExplorerHostEventBridge.decode(json))
    }
  }

  @Test
  fun decodesNullablePayloadFields() {
    val cases =
      listOf(
        """{"name":"pageTitleChanged","payload":{"title":null}}""" to
          ExplorerHostEvent.PageTitleChanged(null),
        """{"name":"statusTextChanged","payload":{"status":null}}""" to
          ExplorerHostEvent.StatusTextChanged(null),
        """{"name":"crashed","payload":{"url":"https://example.com/","reason":"boom","backtrace":null}}""" to
          ExplorerHostEvent.Crashed("https://example.com/", "boom", null),
        """{"name":"error","payload":{"url":null,"code":2,"message":"no current entry"}}""" to
          ExplorerHostEvent.Error(null, 2, "no current entry"),
        """{"name":"simpleDialogRequested","payload":{"dialogId":"dialog-1","kind":"prompt","message":"Name?","defaultValue":null}}""" to
          ExplorerHostEvent.SimpleDialogRequested("dialog-1", "prompt", "Name?", null),
        """{"name":"inputMethodRequested","payload":{"inputMethodId":"ime-1","type":"text","text":"hello","insertionPoint":null,"multiline":false,"allowVirtualKeyboard":true}}""" to
          ExplorerHostEvent.InputMethodRequested(
            inputMethodId = "ime-1",
            type = "text",
            text = "hello",
            insertionPoint = null,
            multiline = false,
            allowVirtualKeyboard = true
          )
      )

    cases.forEach { (json, expected) ->
      assertEquals(expected, ExplorerHostEventBridge.decode(json))
    }
  }

  @Test
  fun decodesNestedBridgePayloads() {
    assertEquals(
      ExplorerHostEvent.SelectElementRequested(
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
      ExplorerHostEventBridge.decode(
        """{"name":"selectElementRequested","payload":{"selectElementId":"select-1","options":[{"type":"option","id":1,"label":"Servo","isDisabled":false},{"type":"optgroup","label":"Engines","options":[{"id":4,"label":"Layout","isDisabled":true}]}],"selectedOptions":[1,4],"allowSelectMultiple":true}}"""
      )
    )

    assertEquals(
      ExplorerHostEvent.ContextMenuRequested(
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
      ExplorerHostEventBridge.decode(
        """{"name":"contextMenuRequested","payload":{"contextMenuId":"context-menu-1","x":12,"y":24,"width":32,"height":48,"elementInfo":{"isLink":true,"isImage":false,"isEditableText":false,"hasSelection":false,"linkUrl":"https://example.com/","imageUrl":null,"contextType":"link"},"items":[{"type":"item","label":"Copy link","action":"copy-link","enabled":true},{"type":"separator"}]}}"""
      )
    )

    assertEquals(
      ExplorerHostEvent.FilePickerRequested(
        filePickerId = "file-picker-1",
        currentPaths = listOf("/cache/one.txt"),
        filterPatterns = listOf("txt", "png"),
        allowSelectMultiple = true
      ),
      ExplorerHostEventBridge.decode(
        """{"name":"filePickerRequested","payload":{"filePickerId":"file-picker-1","currentPaths":["/cache/one.txt"],"filterPatterns":["txt","png"],"allowSelectMultiple":true}}"""
      )
    )
  }
}
