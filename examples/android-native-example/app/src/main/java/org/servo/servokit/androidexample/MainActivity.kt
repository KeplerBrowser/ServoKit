package org.servo.servokit.androidexample

import android.Manifest
import android.app.Activity
import android.app.AlertDialog
import android.content.ActivityNotFoundException
import android.content.ContentResolver
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.text.InputType
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.CheckBox
import android.widget.EditText
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.ScrollView
import android.widget.TextView
import android.webkit.MimeTypeMap
import org.servo.servokit.androidhost.SIMPLE_DIALOG_ACTION_CONFIRM
import org.servo.servokit.androidhost.SIMPLE_DIALOG_ACTION_DISMISS
import java.io.File
import java.io.IOException
import java.util.Locale
import java.util.UUID

class MainActivity : Activity() {
  private lateinit var servoView: NativeServoView
  private lateinit var addressInput: EditText
  private lateinit var eventLogView: TextView
  private val recentHostEvents = mutableListOf<String>()
  private val activeDialogs = mutableMapOf<String, AlertDialog>()
  private var activeSelectElement: NativeSelectElementRequest? = null
  private var activeSelectElementDialog: AlertDialog? = null
  private var activeContextMenu: NativeContextMenuRequest? = null
  private var activeContextMenuDialog: AlertDialog? = null
  private var activeFilePicker: NativeFilePickerRequest? = null
  private var activePermissionRequest: NativePermissionRequest? = null
  private var activePermissionDialog: AlertDialog? = null
  private var pendingAndroidPermissionRequest: NativePermissionRequest? = null
  private val filePickerCacheRoot: File by lazy {
    File(cacheDir, "servo-native-file-picker-${UUID.randomUUID()}")
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    ExampleFixtureServer.start(applicationContext)
    val initialUrl = intent.getStringExtra(EXTRA_INITIAL_URL) ?: DEFAULT_INITIAL_URL

    addressInput =
      EditText(this).apply {
        setSingleLine(true)
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
        setText(initialUrl)
        hint = "Enter URL"
        setOnEditorActionListener { _, _, _ ->
          loadAddressBarUrl()
          true
        }
      }
    eventLogView =
      TextView(this).apply {
        text = "Waiting for Servokit URL/load/history/focus/navigation/dialog/IME/select/file-picker/permission/context-menu events…"
        textSize = 12f
        setPadding(24, 12, 24, 12)
      }
    servoView =
      NativeServoView(this).apply {
        onHostEvent = ::handleHostEvent
      }

    val header =
      TextView(this).apply {
        text = "ServoKit Android host browser\nExperimental Kotlin host example — not a stable Kotlin SDK/AAR"
        textSize = 14f
        setPadding(24, 16, 24, 8)
      }

    val root =
      LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        addView(header, matchWidthWrapHeight())
        addView(createControlPanel(), matchWidthWrapHeight())
        addView(
          servoView,
          LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            0,
            1f
          )
        )
        addView(eventLogView, matchWidthWrapHeight())
      }

    setContentView(root)
    servoView.loadUrl(initialUrl)
  }

  override fun onDestroy() {
    if (::servoView.isInitialized) {
      dismissActiveSelectElementDialog()
      activeSelectElement = null
      dismissActiveContextMenuDialog(sendDismiss = true, reason = "activity destroyed")
      dismissActiveFilePicker(reason = "activity destroyed")
      denyPendingPermissionRequest(reason = "activity destroyed")
      dismissActivePermissionDialog()
      resolveAndDismissActiveDialogs()
      servoView.destroy()
      filePickerCacheRoot.deleteRecursively()
    }
    super.onDestroy()
  }

  @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
  override fun onBackPressed() {
    if (::servoView.isInitialized && servoView.dismissInputMethodIfActive()) {
      return
    }
    super.onBackPressed()
  }

  @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
  override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
    super.onActivityResult(requestCode, resultCode, data)
    if (requestCode == FILE_PICKER_REQUEST_CODE) {
      handleFilePickerActivityResult(resultCode, data)
    }
  }

  override fun onRequestPermissionsResult(
    requestCode: Int,
    permissions: Array<out String>,
    grantResults: IntArray
  ) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults)
    if (requestCode == ANDROID_PERMISSION_REQUEST_CODE) {
      handleAndroidPermissionResult(permissions, grantResults)
    }
  }

  private fun createControlPanel(): LinearLayout =
    LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setPadding(24, 0, 24, 12)
      addView(createAddressRow(), matchWidthWrapHeight())
      addView(
        TextView(this@MainActivity).apply {
          text = "Browser controls use the shared crates/servokit-host-android/android binding: load URL, reload, back, forward, focus, and blur. Touch, text IME, select input, non-text picker input, file input, permission prompts, and context-menu fallback inside the SurfaceView are forwarded through the same Android ServoKit host/control path used below React Native."
          textSize = 12f
        },
        matchWidthWrapHeight()
      )
      addView(horizontalButtonStrip(commandButtons()), matchWidthWrapHeight())
      addView(
        TextView(this@MainActivity).apply {
          text = "Readiness fixtures"
          textSize = 12f
        },
        matchWidthWrapHeight()
      )
      addView(horizontalButtonStrip(fixtureButtons(READINESS_FIXTURES)), matchWidthWrapHeight())
      addView(
        TextView(this@MainActivity).apply {
          text = "Embedder control fixtures"
          textSize = 12f
        },
        matchWidthWrapHeight()
      )
      addView(horizontalButtonStrip(fixtureButtons(CONTROL_FIXTURES)), matchWidthWrapHeight())
    }

  private fun createAddressRow(): LinearLayout =
    LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER_VERTICAL
      addView(
        addressInput,
        LinearLayout.LayoutParams(
          0,
          ViewGroup.LayoutParams.WRAP_CONTENT,
          1f
        )
      )
      addView(
        createButton("Load") { loadAddressBarUrl() },
        LinearLayout.LayoutParams(
          ViewGroup.LayoutParams.WRAP_CONTENT,
          ViewGroup.LayoutParams.WRAP_CONTENT
        )
      )
    }

  private fun commandButtons(): List<Button> =
    listOf(
      createButton("Reload") { servoView.reload() },
      createButton("Back") { servoView.goBack() },
      createButton("Forward") { servoView.goForward() },
      createButton("Focus") { servoView.focusWebView() },
      createButton("Blur") { servoView.blurWebView() }
    )

  private fun fixtureButtons(fixtures: List<FixturePage>): List<Button> =
    fixtures.map { fixture ->
      createButton(fixture.label) { navigateTo(fixture.url) }
    }

  private fun horizontalButtonStrip(buttons: List<Button>): HorizontalScrollView =
    HorizontalScrollView(this).apply {
      isHorizontalScrollBarEnabled = false
      addView(
        LinearLayout(this@MainActivity).apply {
          orientation = LinearLayout.HORIZONTAL
          buttons.forEach { button -> addView(button) }
        },
        ViewGroup.LayoutParams(
          ViewGroup.LayoutParams.WRAP_CONTENT,
          ViewGroup.LayoutParams.WRAP_CONTENT
        )
      )
    }

  private fun createButton(label: String, action: () -> Unit): Button =
    Button(this).apply {
      text = label
      setOnClickListener { action() }
    }

  private fun loadAddressBarUrl() {
    navigateTo(addressInput.text.toString())
  }

  private fun navigateTo(url: String) {
    val trimmedUrl = url.trim()
    if (trimmedUrl.isEmpty()) {
      return
    }

    addressInput.setText(trimmedUrl)
    addressInput.setSelection(addressInput.text.length)
    servoView.loadUrl(trimmedUrl)
  }

  private fun handleHostEvent(event: NativeHostEvent) {
    val line = event.describe()
    Log.i(TAG, line)

    if (event.name in DISPLAYED_EVENT_NAMES) {
      appendEventLogLine(line)
    }

    when (event.name) {
      "urlChanged" -> updateAddressFromEvent(event)
      "navigationRequested" -> event.navigationRequest()?.let(::handleNavigationRequested)
      "simpleDialogRequested" -> event.simpleDialogRequest()?.let(::showSimpleDialog)
      "simpleDialogDismissed" -> event.simpleDialogDismissedId()?.let(::dismissSimpleDialog)
      "selectElementRequested" -> event.selectElementRequest()?.let(::showSelectElementDialog)
      "selectElementDismissed" -> event.selectElementDismissedId()?.let(::dismissSelectElementDialog)
      "filePickerRequested" -> event.filePickerRequest()?.let(::showFilePicker)
      "filePickerDismissed" -> event.filePickerDismissedId()?.let(::handleFilePickerDismissed)
      "permissionRequested" -> event.permissionRequest()?.let(::handlePermissionRequested)
      "contextMenuRequested" -> event.contextMenuRequest()?.let(::showContextMenuDialog)
      "contextMenuDismissed" -> event.contextMenuDismissedId()?.let(::dismissContextMenuDialog)
    }
  }

  private fun updateAddressFromEvent(event: NativeHostEvent) {
    val url = event.urlChanged() ?: return
    if (url.isNotBlank()) {
      addressInput.setText(url)
      addressInput.setSelection(addressInput.text.length)
    }
  }

  private fun handleNavigationRequested(request: NativeNavigationRequest) {
    val allow = shouldAllowNavigation(request.url)
    val decision = if (allow) "allow" else "deny"
    val line = "policy=$decision url=${request.url}"
    Log.i(TAG, line)
    appendEventLogLine(line)
    servoView.resolveNavigationRequest(request.navigationId, allow)
  }

  private fun shouldAllowNavigation(url: String): Boolean {
    val normalizedUrl = url.lowercase(Locale.ROOT)
    if ("blocked-by-policy" in normalizedUrl) {
      return false
    }

    val scheme = Uri.parse(url).scheme?.lowercase(Locale.ROOT)
    return scheme == "http" || scheme == "https"
  }

  private fun showSimpleDialog(request: NativeSimpleDialogRequest) {
    dismissSimpleDialog(request.dialogId)

    val builder =
      AlertDialog.Builder(this)
        .setTitle("JavaScript ${request.kind.ifBlank { "dialog" }}")
        .setMessage(request.message)
        .setCancelable(false)

    when (request.kind) {
      "alert" -> {
        builder.setPositiveButton(android.R.string.ok) { _, _ ->
          resolveSimpleDialog(request.dialogId, SIMPLE_DIALOG_ACTION_CONFIRM, null)
        }
      }
      "confirm" -> {
        builder
          .setPositiveButton(android.R.string.ok) { _, _ ->
            resolveSimpleDialog(request.dialogId, SIMPLE_DIALOG_ACTION_CONFIRM, null)
          }
          .setNegativeButton(android.R.string.cancel) { _, _ ->
            resolveSimpleDialog(request.dialogId, SIMPLE_DIALOG_ACTION_DISMISS, null)
          }
      }
      "prompt" -> {
        val promptField =
          EditText(this).apply {
            inputType = InputType.TYPE_CLASS_TEXT
            setSingleLine(true)
            setText(request.defaultValue.orEmpty())
            setSelection(text.length)
          }
        builder
          .setView(promptField)
          .setPositiveButton(android.R.string.ok) { _, _ ->
            resolveSimpleDialog(
              request.dialogId,
              SIMPLE_DIALOG_ACTION_CONFIRM,
              promptField.text?.toString().orEmpty()
            )
          }
          .setNegativeButton(android.R.string.cancel) { _, _ ->
            resolveSimpleDialog(request.dialogId, SIMPLE_DIALOG_ACTION_DISMISS, null)
          }
      }
      else -> {
        val line = "dialog unsupported kind=${request.kind} id=${request.dialogId}; dismissing"
        Log.w(TAG, line)
        appendEventLogLine(line)
        servoView.resolveSimpleDialog(request.dialogId, SIMPLE_DIALOG_ACTION_DISMISS, null)
        return
      }
    }

    if (isFinishing || isDestroyed) {
      servoView.resolveSimpleDialog(request.dialogId, SIMPLE_DIALOG_ACTION_DISMISS, null)
      return
    }

    val dialog = builder.create()
    dialog.setOnDismissListener {
      activeDialogs.remove(request.dialogId)
    }
    activeDialogs[request.dialogId] = dialog
    dialog.show()
  }

  private fun resolveSimpleDialog(dialogId: String, action: Int, promptValue: String?) {
    activeDialogs.remove(dialogId)
    servoView.resolveSimpleDialog(dialogId, action, promptValue)
  }

  private fun dismissSimpleDialog(dialogId: String) {
    activeDialogs.remove(dialogId)?.let { dialog ->
      dialog.setOnDismissListener(null)
      dialog.dismiss()
    }
  }

  private fun resolveAndDismissActiveDialogs() {
    val dialogs = activeDialogs.toMap()
    activeDialogs.clear()
    dialogs.keys.forEach { dialogId ->
      servoView.resolveSimpleDialog(dialogId, SIMPLE_DIALOG_ACTION_DISMISS, null)
    }
    dialogs.values.forEach { dialog ->
      dialog.setOnDismissListener(null)
      dialog.dismiss()
    }
  }

  private fun showSelectElementDialog(request: NativeSelectElementRequest) {
    if (activeSelectElement?.selectElementId == request.selectElementId && activeSelectElementDialog != null) {
      return
    }

    dismissActiveSelectElementDialog()
    activeSelectElement = request

    if (isFinishing || isDestroyed) {
      resolveSelectElement(request, request.selectedOptions.toIntArray())
      return
    }

    val optionButtons = mutableListOf<Pair<Int, TextView>>()
    val initialSelectedOptionId = request.selectedOptions.firstOrNull()
    val initialSelectedOptions = request.selectedOptions.toSet()
    val content =
      LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(16), dp(8), dp(16), 0)
      }

    fun displayLabel(option: NativeSelectElementOption): String {
      val label = option.label.ifBlank { "(blank option)" }
      return if (option.isDisabled) "$label (disabled)" else label
    }

    fun addOption(option: NativeSelectElementOption, nested: Boolean) {
      val button =
        if (request.allowSelectMultiple) {
          CheckBox(this)
        } else {
          RadioButton(this)
        }.apply {
          text = displayLabel(option)
          isEnabled = !option.isDisabled
          isChecked =
            if (request.allowSelectMultiple) {
              initialSelectedOptions.contains(option.id)
            } else {
              option.id == initialSelectedOptionId
            }
          if (nested) {
            setPadding(dp(12), paddingTop, paddingRight, paddingBottom)
          }
        }
      if (!request.allowSelectMultiple) {
        button.setOnClickListener {
          optionButtons.forEach { (_, candidate) ->
            if (candidate !== button) {
              (candidate as? RadioButton)?.isChecked = false
            }
          }
          (button as RadioButton).isChecked = true
        }
      }
      optionButtons += option.id to button
      content.addView(button)
    }

    for (item in request.options) {
      when (item) {
        is NativeSelectElementOptionOrOptgroup.Option -> addOption(item.option, nested = false)
        is NativeSelectElementOptionOrOptgroup.Optgroup -> {
          content.addView(
            TextView(this).apply {
              text = "Group: ${item.label.ifBlank { "(unnamed)" }}"
              textSize = 12f
              setPadding(0, dp(8), 0, dp(4))
            }
          )
          item.options.forEach { option -> addOption(option, nested = true) }
        }
      }
    }

    if (optionButtons.isEmpty()) {
      content.addView(
        TextView(this).apply {
          text = "No selectable options were reported by the host event."
          setPadding(0, dp(8), 0, dp(8))
        }
      )
    }

    val scrollView =
      ScrollView(this).apply {
        addView(
          content,
          ViewGroup.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
          )
        )
      }
    var resolved = false
    fun currentSelection(): IntArray =
      optionButtons
        .filter { (_, button) ->
          when (button) {
            is CheckBox -> button.isChecked
            is RadioButton -> button.isChecked
            else -> false
          }
        }
        .map { (id, _) -> id }
        .toIntArray()
    fun resolveSelection(selectedOptions: IntArray) {
      if (resolved || activeSelectElement?.selectElementId != request.selectElementId) {
        return
      }

      resolved = true
      resolveSelectElement(request, selectedOptions)
    }

    val dialog =
      AlertDialog.Builder(this)
        .setTitle(if (request.allowSelectMultiple) "Select options" else "Select an option")
        .setView(scrollView)
        .setPositiveButton(android.R.string.ok) { _, _ ->
          resolveSelection(currentSelection())
        }
        .setNegativeButton(android.R.string.cancel) { _, _ ->
          resolveSelection(request.selectedOptions.toIntArray())
        }
        .create()
    activeSelectElementDialog = dialog
    dialog.setOnDismissListener {
      if (activeSelectElementDialog === dialog) {
        activeSelectElementDialog = null
      }
      if (!resolved) {
        resolveSelection(request.selectedOptions.toIntArray())
      }
    }
    dialog.show()
  }

  private fun resolveSelectElement(request: NativeSelectElementRequest, selectedOptions: IntArray) {
    if (activeSelectElement?.selectElementId == request.selectElementId) {
      activeSelectElement = null
    }
    val line =
      "select resolved id=${request.selectElementId} selected=${selectedOptions.joinToString(prefix = "[", postfix = "]")}"
    Log.i(TAG, line)
    appendEventLogLine(line)
    servoView.resolveSelectElement(request.selectElementId, selectedOptions)
  }

  private fun dismissSelectElementDialog(selectElementId: String) {
    if (activeSelectElement?.selectElementId != selectElementId) {
      return
    }

    dismissActiveSelectElementDialog()
    activeSelectElement = null
  }

  private fun dismissActiveSelectElementDialog() {
    val dialog = activeSelectElementDialog ?: return
    activeSelectElementDialog = null
    dialog.setOnDismissListener(null)
    dialog.dismiss()
  }

  private fun showFilePicker(request: NativeFilePickerRequest) {
    if (activeFilePicker?.filePickerId == request.filePickerId) {
      return
    }

    dismissActiveFilePicker(reason = "superseded")
    activeFilePicker = request

    if (isFinishing || isDestroyed) {
      dismissActiveFilePicker(reason = "activity not active")
      return
    }

    val mimeTypes = mimeTypesForFilterPatterns(request.filterPatterns)
    val launchLine =
      "file picker launching id=${request.filePickerId} multiple=${request.allowSelectMultiple} filters=${request.filterPatterns} mimeTypes=${mimeTypes.ifEmpty { listOf("*/*") }} currentPaths=${request.currentPaths.size}"
    Log.i(TAG, launchLine)
    appendEventLogLine(launchLine)

    val intent =
      Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
        addCategory(Intent.CATEGORY_OPENABLE)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        putExtra(Intent.EXTRA_ALLOW_MULTIPLE, request.allowSelectMultiple)
        when (mimeTypes.size) {
          0 -> type = "*/*"
          1 -> type = mimeTypes.single()
          else -> {
            type = "*/*"
            putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toTypedArray())
          }
        }
      }

    try {
      startActivityForResult(intent, FILE_PICKER_REQUEST_CODE)
    } catch (error: ActivityNotFoundException) {
      val line = "file picker unavailable id=${request.filePickerId}: ${error.message.orEmpty()}"
      Log.w(TAG, line, error)
      appendEventLogLine(line)
      dismissActiveFilePicker(reason = "no activity")
    }
  }

  private fun handleFilePickerDismissed(filePickerId: String) {
    if (activeFilePicker?.filePickerId != filePickerId) {
      return
    }

    activeFilePicker = null
  }

  private fun handleFilePickerActivityResult(resultCode: Int, data: Intent?) {
    val request = activeFilePicker ?: return
    if (resultCode != Activity.RESULT_OK) {
      dismissActiveFilePicker(reason = "cancelled")
      return
    }

    val selectedUris = extractSelectedUris(data, request.allowSelectMultiple)
    if (selectedUris.isEmpty()) {
      dismissActiveFilePicker(reason = "empty selection")
      return
    }

    Thread {
      val copiedPaths = copySelectedUrisToCache(selectedUris)
      runOnUiThread {
        if (!::servoView.isInitialized || activeFilePicker?.filePickerId != request.filePickerId) {
          return@runOnUiThread
        }
        if (copiedPaths == null || copiedPaths.isEmpty()) {
          dismissActiveFilePicker(reason = "copy failed")
          return@runOnUiThread
        }

        activeFilePicker = null
        val selectedPaths = copiedPaths.toTypedArray()
        val selectedSummary = selectedPaths.joinToString(prefix = "[", postfix = "]")
        val line = "file picker resolved id=${request.filePickerId} selected=$selectedSummary"
        Log.i(TAG, line)
        appendEventLogLine(line)
        servoView.resolveFilePicker(request.filePickerId, selectedPaths)
      }
    }.start()
  }

  private fun dismissActiveFilePicker(reason: String) {
    val request = activeFilePicker ?: return
    activeFilePicker = null
    val line = "file picker dismissed by native id=${request.filePickerId} reason=$reason"
    Log.i(TAG, line)
    appendEventLogLine(line)
    servoView.dismissFilePicker(request.filePickerId)
  }

  private fun extractSelectedUris(data: Intent?, allowSelectMultiple: Boolean): List<Uri> {
    if (data == null) {
      return emptyList()
    }

    val uris = mutableListOf<Uri>()
    data.data?.let(uris::add)
    val clipData = data.clipData
    if (clipData != null) {
      for (index in 0 until clipData.itemCount) {
        clipData.getItemAt(index)?.uri?.let(uris::add)
      }
    }
    val distinctUris = uris.distinct()
    return if (allowSelectMultiple) {
      distinctUris
    } else {
      distinctUris.take(1)
    }
  }

  private fun copySelectedUrisToCache(selectedUris: List<Uri>): List<String>? {
    val resolver = contentResolver
    val destinationDir =
      File(filePickerCacheRoot, "selection-${System.currentTimeMillis()}").apply {
        mkdirs()
      }
    if (!destinationDir.isDirectory) {
      return null
    }

    val copiedFiles = mutableListOf<String>()
    try {
      for ((index, uri) in selectedUris.withIndex()) {
        val destinationFile = File(destinationDir, "${index}-${displayNameForUri(resolver, uri, index)}")
        val input =
          resolver.openInputStream(uri) ?: run {
            destinationDir.deleteRecursively()
            return null
          }
        input.use {
          destinationFile.outputStream().use { output ->
            it.copyTo(output)
          }
        }
        copiedFiles += destinationFile.absolutePath
      }
    } catch (_: IOException) {
      destinationDir.deleteRecursively()
      return null
    } catch (_: SecurityException) {
      destinationDir.deleteRecursively()
      return null
    }

    return copiedFiles
  }

  private fun displayNameForUri(resolver: ContentResolver, uri: Uri, index: Int): String {
    val displayName =
      resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        val columnIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
        if (columnIndex >= 0 && cursor.moveToFirst()) {
          cursor.getString(columnIndex)
        } else {
          null
        }
      }

    val fallbackName =
      when {
        !displayName.isNullOrBlank() -> displayName
        !uri.lastPathSegment.isNullOrBlank() -> uri.lastPathSegment.orEmpty()
        else -> buildString {
          append("selected-file-")
          append(index)
          resolver.getType(uri)
            ?.let(MimeTypeMap.getSingleton()::getExtensionFromMimeType)
            ?.takeIf { it.isNotBlank() }
            ?.let { extension ->
              append('.')
              append(extension)
            }
        }
      }

    return fallbackName
      .replace(Regex("""[\\/:*?"<>|]"""), "_")
      .ifBlank { "selected-file-$index" }
  }

  private fun mimeTypesForFilterPatterns(filterPatterns: List<String>): List<String> {
    val mimeTypes =
      filterPatterns
        .flatMap { pattern -> pattern.split(',') }
        .mapNotNull(::mimeTypeForFilterPattern)
        .distinct()
    return if (mimeTypes.contains("*/*")) {
      emptyList()
    } else {
      mimeTypes
    }
  }

  private fun mimeTypeForFilterPattern(pattern: String): String? {
    val normalizedPattern = pattern.trim().lowercase(Locale.US)
    if (normalizedPattern.isBlank()) {
      return null
    }
    if (normalizedPattern.contains('/')) {
      return normalizedPattern
    }

    val extension = normalizedPattern.removePrefix("*.").removePrefix(".")
    if (extension.isBlank()) {
      return null
    }

    return MimeTypeMap
      .getSingleton()
      .getMimeTypeFromExtension(extension)
      ?: FILE_PICKER_EXTENSION_MIME_TYPES[extension]
  }

  private fun handlePermissionRequested(event: NativePermissionRequest) {
    val pendingRequest = servoView.getPendingPermissionRequest()
    if (pendingRequest == null) {
      val line =
        "permission requested kind=${event.permission} origin=${event.origin} but no host request is pending; ignoring stale event"
      Log.w(TAG, line)
      appendEventLogLine(line)
      return
    }

    if (pendingRequest != event) {
      val line =
        "permission requested event kind=${event.permission} origin=${event.origin} superseded by pending kind=${pendingRequest.permission} origin=${pendingRequest.origin}"
      Log.i(TAG, line)
      appendEventLogLine(line)
    }

    showPermissionDialog(pendingRequest)
  }

  private fun showPermissionDialog(request: NativePermissionRequest) {
    if (activePermissionRequest == request && activePermissionDialog != null) {
      return
    }

    dismissActivePermissionDialog()
    activePermissionRequest = request

    if (isFinishing || isDestroyed) {
      resolvePermissionRequest(request, allow = false, reason = "activity not active")
      return
    }

    var resolved = false
    fun resolve(allow: Boolean, reason: String) {
      if (resolved || activePermissionRequest != request) {
        return
      }

      resolved = true
      resolvePermissionRequest(request, allow, reason)
    }

    fun allowAfterAndroidPermission() {
      if (resolved || activePermissionRequest != request) {
        return
      }

      val missingAndroidPermissions = missingAndroidPermissionsForPermission(request.permission)
      if (missingAndroidPermissions.isEmpty()) {
        resolve(allow = true, reason = "allow")
        return
      }

      resolved = true
      requestAndroidPermissions(request, missingAndroidPermissions)
    }

    val dialog =
      AlertDialog.Builder(this)
        .setTitle(permissionDialogTitle(request.permission))
        .setMessage(permissionDialogMessage(request))
        .setPositiveButton("Allow") { _, _ ->
          allowAfterAndroidPermission()
        }
        .setNegativeButton("Deny") { _, _ ->
          resolve(allow = false, reason = "deny")
        }
        .create()
    activePermissionDialog = dialog
    dialog.setOnDismissListener {
      if (activePermissionDialog === dialog) {
        activePermissionDialog = null
      }
      if (!resolved) {
        resolve(allow = false, reason = "dismissed")
      }
    }
    dialog.setOnCancelListener {
      resolve(allow = false, reason = "cancel")
    }
    dialog.show()
  }

  private fun requestAndroidPermissions(
    request: NativePermissionRequest,
    permissions: List<String>
  ) {
    pendingAndroidPermissionRequest = request
    val line =
      "permission android-runtime requested kind=${request.permission} permissions=${permissions.map(::androidPermissionLabel)}"
    Log.i(TAG, line)
    appendEventLogLine(line)
    requestPermissions(permissions.toTypedArray(), ANDROID_PERMISSION_REQUEST_CODE)
  }

  private fun handleAndroidPermissionResult(
    permissions: Array<out String>,
    grantResults: IntArray
  ) {
    val request = pendingAndroidPermissionRequest ?: return
    pendingAndroidPermissionRequest = null

    val resultSummary =
      permissions
        .mapIndexed { index, permission ->
          val granted = grantResults.getOrNull(index) == PackageManager.PERMISSION_GRANTED
          "${androidPermissionLabel(permission)}=${if (granted) "granted" else "denied"}"
        }
        .joinToString(prefix = "[", postfix = "]")
    val allow = isAndroidPermissionSatisfied(request.permission)
    val line =
      "permission android-runtime result kind=${request.permission} allow=$allow results=$resultSummary"
    Log.i(TAG, line)
    appendEventLogLine(line)
    resolvePermissionRequest(
      request,
      allow = allow,
      reason = if (allow) "android runtime granted" else "android runtime denied"
    )
  }

  private fun resolvePermissionRequest(
    request: NativePermissionRequest,
    allow: Boolean,
    reason: String
  ) {
    if (activePermissionRequest == request) {
      activePermissionRequest = null
    }
    if (pendingAndroidPermissionRequest == request) {
      pendingAndroidPermissionRequest = null
    }

    val pendingRequest = servoView.getPendingPermissionRequest()
    if (pendingRequest == null) {
      val line =
        "permission resolution skipped kind=${request.permission} origin=${request.origin} reason=$reason; no pending host request"
      Log.w(TAG, line)
      appendEventLogLine(line)
      return
    }

    if (pendingRequest != request) {
      val line =
        "permission resolution skipped stale kind=${request.permission} origin=${request.origin}; pending kind=${pendingRequest.permission} origin=${pendingRequest.origin}"
      Log.w(TAG, line)
      appendEventLogLine(line)
      if (!isFinishing && !isDestroyed) {
        showPermissionDialog(pendingRequest)
      }
      return
    }

    val decision = if (allow) "allow" else "deny"
    val line =
      "permission resolved kind=${request.permission} origin=${request.origin} decision=$decision reason=$reason"
    Log.i(TAG, line)
    appendEventLogLine(line)
    servoView.resolvePermission(allow)
  }

  private fun denyPendingPermissionRequest(reason: String) {
    pendingAndroidPermissionRequest = null
    val request = activePermissionRequest ?: servoView.getPendingPermissionRequest()
    if (request == null && !servoView.hasPendingPermissionRequest()) {
      return
    }

    activePermissionRequest = null
    val line =
      request?.let {
        "permission resolved kind=${it.permission} origin=${it.origin} decision=deny reason=$reason"
      } ?: "permission resolved decision=deny reason=$reason"
    Log.i(TAG, line)
    appendEventLogLine(line)
    servoView.resolvePermission(false)
  }

  private fun dismissActivePermissionDialog() {
    val dialog = activePermissionDialog ?: return
    activePermissionDialog = null
    dialog.setOnDismissListener(null)
    dialog.setOnCancelListener(null)
    dialog.dismiss()
  }

  private fun permissionDialogTitle(permission: String): String =
    if (permission.isBlank()) {
      "Permission request"
    } else {
      "${permissionKindLabel(permission)} permission"
    }

  private fun permissionKindLabel(permission: String): String =
    permission
      .trim()
      .replace('-', ' ')
      .replace('_', ' ')
      .replaceFirstChar { char ->
        if (char.isLowerCase()) {
          char.titlecase(Locale.US)
        } else {
          char.toString()
        }
      }
      .ifBlank { "Permission" }

  private fun permissionDialogMessage(request: NativePermissionRequest): String {
    val androidRequirement = androidPermissionRequirementFor(request.permission)
    val androidPermissionLine =
      if (androidRequirement == null) {
        "No Android runtime permission is required or mapped for this proof permission kind."
      } else {
        val labels = androidRequirement.permissions.map(::androidPermissionLabel).joinToString()
        val mode = if (androidRequirement.allowWhenAnyGranted) "at least one of" else "all of"
        "Allow may first request Android runtime permission for $mode: $labels."
      }

    return buildString {
      append("Origin: ")
      append(request.origin.ifBlank { "(unknown origin)" })
      append('\n')
      append("Permission: ")
      append(request.permission.ifBlank { "(unknown kind)" })
      append("\n\n")
      append(androidPermissionLine)
      append("\n\nAllow and Deny resolve through the shared ServoViewBinding permission path.")
    }
  }

  private fun missingAndroidPermissionsForPermission(permission: String): List<String> {
    val requirement = androidPermissionRequirementFor(permission) ?: return emptyList()
    if (isAndroidPermissionRequirementSatisfied(requirement)) {
      return emptyList()
    }

    return requirement.permissions.filterNot(::isAndroidPermissionGranted)
  }

  private fun isAndroidPermissionSatisfied(permission: String): Boolean {
    val requirement = androidPermissionRequirementFor(permission) ?: return true
    return isAndroidPermissionRequirementSatisfied(requirement)
  }

  private fun isAndroidPermissionRequirementSatisfied(
    requirement: AndroidPermissionRequirement
  ): Boolean =
    if (requirement.allowWhenAnyGranted) {
      requirement.permissions.any(::isAndroidPermissionGranted)
    } else {
      requirement.permissions.all(::isAndroidPermissionGranted)
    }

  private fun isAndroidPermissionGranted(permission: String): Boolean =
    Build.VERSION.SDK_INT < Build.VERSION_CODES.M ||
      checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

  private fun androidPermissionRequirementFor(permission: String): AndroidPermissionRequirement? =
    when (permission.trim().lowercase(Locale.US)) {
      "geolocation",
      "location" ->
        AndroidPermissionRequirement(
          permissions =
            listOf(
              Manifest.permission.ACCESS_FINE_LOCATION,
              Manifest.permission.ACCESS_COARSE_LOCATION
            ),
          allowWhenAnyGranted = true
        )
      "camera",
      "video-capture" -> AndroidPermissionRequirement(listOf(Manifest.permission.CAMERA))
      "microphone",
      "audio-capture" -> AndroidPermissionRequirement(listOf(Manifest.permission.RECORD_AUDIO))
      "notification",
      "notifications" ->
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
          AndroidPermissionRequirement(listOf(Manifest.permission.POST_NOTIFICATIONS))
        } else {
          null
        }
      else -> null
    }

  private fun androidPermissionLabel(permission: String): String =
    permission.substringAfterLast('.')

  private fun showContextMenuDialog(request: NativeContextMenuRequest) {
    if (activeContextMenu?.contextMenuId == request.contextMenuId && activeContextMenuDialog != null) {
      return
    }

    dismissActiveContextMenuDialog(sendDismiss = true, reason = "superseded")
    activeContextMenu = request

    if (isFinishing || isDestroyed) {
      activeContextMenu = null
      dismissContextMenuRequest(request, "activity not active")
      return
    }

    data class ContextMenuDialogItem(
      val title: String,
      val subtitle: String?,
      val action: String?,
      val enabled: Boolean,
      val separator: Boolean = false,
      val header: Boolean = false
    )

    fun toActionRows(items: List<NativeContextMenuItem>): List<ContextMenuDialogItem> {
      val rows = mutableListOf<ContextMenuDialogItem>()
      var previousWasSeparator = true
      for (item in items) {
        when (item) {
          is NativeContextMenuItem.Item -> {
            val action = item.action.trim()
            val enabled = item.enabled && action.isNotEmpty()
            val title = item.label.ifBlank { action.ifBlank { "(unlabeled action)" } }
            rows +=
              ContextMenuDialogItem(
                title = if (item.enabled) title else "$title (disabled)",
                subtitle =
                  if (action.isEmpty()) {
                    "Servo item missing an action id"
                  } else if (enabled) {
                    "Servo action: $action"
                  } else {
                    "Disabled Servo action: $action"
                  },
                action = action.takeIf { it.isNotEmpty() },
                enabled = enabled
              )
            previousWasSeparator = false
          }
          NativeContextMenuItem.Separator -> {
            if (!previousWasSeparator) {
              rows +=
                ContextMenuDialogItem(
                  title = "────────",
                  subtitle = null,
                  action = null,
                  enabled = false,
                  separator = true
                )
              previousWasSeparator = true
            }
          }
        }
      }
      while (rows.lastOrNull()?.separator == true) {
        rows.removeAt(rows.lastIndex)
      }
      return rows
    }

    val actionRows = toActionRows(request.items)
    val dialogItems =
      buildList {
        contextMenuSummaryLines(request).forEach { line ->
          add(
            ContextMenuDialogItem(
              title = line,
              subtitle = null,
              action = null,
              enabled = false,
              header = true
            )
          )
        }
        add(
          ContextMenuDialogItem(
            title = "────────",
            subtitle = null,
            action = null,
            enabled = false,
            separator = true
          )
        )
        add(
          ContextMenuDialogItem(
            title = "Servo actions (${request.itemCount()} items)",
            subtitle = "Provided by the shared contextMenuRequested payload",
            action = null,
            enabled = false,
            header = true
          )
        )
        if (actionRows.isEmpty()) {
          add(
            ContextMenuDialogItem(
              title = "No enabled Servo actions reported",
              subtitle = "Cancel dismisses through the shared ServoViewBinding context-menu path",
              action = null,
              enabled = false
            )
          )
        } else {
          addAll(actionRows)
        }
      }

    val adapter =
      object : ArrayAdapter<ContextMenuDialogItem>(
        this@MainActivity,
        android.R.layout.simple_list_item_2,
        dialogItems
      ) {
        override fun areAllItemsEnabled(): Boolean = false

        override fun isEnabled(position: Int): Boolean {
          val item = dialogItems[position]
          return item.enabled && item.action != null
        }

        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
          val view = super.getView(position, convertView, parent)
          val item = dialogItems[position]
          val titleView = view.findViewById<TextView>(android.R.id.text1)
          val subtitleView = view.findViewById<TextView>(android.R.id.text2)
          titleView.text = item.title
          subtitleView.text = item.subtitle.orEmpty()
          subtitleView.visibility = if (item.subtitle == null) View.GONE else View.VISIBLE

          val rowEnabled = isEnabled(position)
          titleView.isEnabled = rowEnabled
          subtitleView.isEnabled = rowEnabled
          when {
            item.header -> titleView.setTextColor(Color.parseColor("#64748B"))
            item.separator -> titleView.setTextColor(Color.parseColor("#CBD5E1"))
            rowEnabled -> titleView.setTextColor(Color.parseColor("#0F172A"))
            else -> titleView.setTextColor(Color.parseColor("#94A3B8"))
          }
          return view
        }
      }

    var resolved = false
    fun resolve(dialogItem: ContextMenuDialogItem) {
      val action = dialogItem.action ?: return
      if (!dialogItem.enabled || resolved || activeContextMenu?.contextMenuId != request.contextMenuId) {
        return
      }

      resolved = true
      activeContextMenu = null
      val dialog = activeContextMenuDialog
      activeContextMenuDialog = null
      if (dialog != null) {
        dialog.setOnDismissListener(null)
        dialog.setOnCancelListener(null)
        dialog.dismiss()
      }
      val line = "context menu resolved id=${request.contextMenuId} action=$action label=${dialogItem.title}"
      Log.i(TAG, line)
      appendEventLogLine(line)
      servoView.resolveContextMenu(request.contextMenuId, action)
    }

    fun dismiss(reason: String) {
      if (resolved || activeContextMenu?.contextMenuId != request.contextMenuId) {
        return
      }

      resolved = true
      activeContextMenu = null
      dismissContextMenuRequest(request, reason)
    }

    val dialog =
      AlertDialog.Builder(this)
        .setTitle("${contextMenuContextLabel(request.elementInfo.contextType)} menu")
        .setAdapter(adapter) { _, which ->
          dialogItems.getOrNull(which)?.let(::resolve)
        }
        .setNegativeButton(android.R.string.cancel) { _, _ ->
          dismiss("cancel")
        }
        .create()
    activeContextMenuDialog = dialog
    dialog.setOnDismissListener {
      if (activeContextMenuDialog === dialog) {
        activeContextMenuDialog = null
      }
      if (!resolved) {
        dismiss("dismissed")
      }
    }
    dialog.setOnCancelListener {
      dismiss("cancel")
    }
    dialog.show()
  }

  private fun dismissContextMenuDialog(contextMenuId: String) {
    if (activeContextMenu?.contextMenuId != contextMenuId) {
      return
    }

    dismissActiveContextMenuDialog(sendDismiss = false, reason = "host dismissed")
  }

  private fun dismissActiveContextMenuDialog(sendDismiss: Boolean, reason: String) {
    val request = activeContextMenu
    activeContextMenu = null

    val dialog = activeContextMenuDialog
    activeContextMenuDialog = null
    if (dialog != null) {
      dialog.setOnDismissListener(null)
      dialog.setOnCancelListener(null)
      dialog.dismiss()
    }

    if (sendDismiss && request != null) {
      dismissContextMenuRequest(request, reason)
    }
  }

  private fun dismissContextMenuRequest(request: NativeContextMenuRequest, reason: String) {
    val line = "context menu dismissed by native id=${request.contextMenuId} reason=$reason"
    Log.i(TAG, line)
    appendEventLogLine(line)
    servoView.dismissContextMenu(request.contextMenuId)
  }

  private fun contextMenuSummaryLines(request: NativeContextMenuRequest): List<String> =
    buildList {
      add("Type: ${contextMenuContextLabel(request.elementInfo.contextType)} (${request.elementInfo.contextType})")
      add("Position: ${request.x},${request.y}; bounds: ${request.width}x${request.height}")
      add(
        "Flags: " +
          contextMenuFlags(request.elementInfo).joinToString().ifBlank { "generic/page target" }
      )
      compactContextMenuValue(request.elementInfo.linkUrl)?.let { add("Link URL: $it") }
      compactContextMenuValue(request.elementInfo.imageUrl)?.let { add("Image URL: $it") }
    }

  private fun contextMenuFlags(element: NativeContextMenuElementInformation): List<String> =
    buildList {
      if (element.isLink) add("link")
      if (element.isImage) add("image")
      if (element.isEditableText) add("editable text")
      if (element.hasSelection) add("selected text")
    }

  private fun compactContextMenuValue(value: String?): String? {
    val trimmed = value?.trim()?.takeIf { it.isNotEmpty() } ?: return null
    return if (trimmed.length <= MAX_CONTEXT_MENU_VALUE_LENGTH) {
      trimmed
    } else {
      trimmed.take(MAX_CONTEXT_MENU_VALUE_LENGTH - 1) + "…"
    }
  }

  private fun contextMenuContextLabel(contextType: String): String =
    when (contextType) {
      "link" -> "Link"
      "image" -> "Image"
      "media" -> "Media"
      "input" -> "Input"
      "text" -> "Text"
      else -> "Page"
    }

  private fun appendEventLogLine(line: String) {
    recentHostEvents += line
    while (recentHostEvents.size > MAX_DISPLAYED_EVENTS) {
      recentHostEvents.removeAt(0)
    }
    eventLogView.text = recentHostEvents.joinToString(separator = "\n")
  }

  private fun matchWidthWrapHeight(): LinearLayout.LayoutParams =
    LinearLayout.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      ViewGroup.LayoutParams.WRAP_CONTENT
    )

  private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

  private data class AndroidPermissionRequirement(
    val permissions: List<String>,
    val allowWhenAnyGranted: Boolean = false
  )

  companion object {
    private const val TAG = "ServokitNativeExample"
    private const val MAX_DISPLAYED_EVENTS = 8
    private const val MAX_CONTEXT_MENU_VALUE_LENGTH = 120
    const val EXTRA_INITIAL_URL = "org.servo.servokit.androidexample.INITIAL_URL"
    const val DEFAULT_INITIAL_URL = "http://127.0.0.1:8481/smoke/index.html"

    private val READINESS_FIXTURES =
      listOf(
        FixturePage("Smoke", "${ExampleFixtureServer.baseUrl}/smoke/index.html"),
        FixturePage("Title", "${ExampleFixtureServer.baseUrl}/smoke/title-change.html"),
        FixturePage("History", "${ExampleFixtureServer.baseUrl}/smoke/history-start.html"),
        FixturePage("Reload", "${ExampleFixtureServer.baseUrl}/smoke/reload.html"),
        FixturePage("Form", "${ExampleFixtureServer.baseUrl}/smoke/form.html"),
        FixturePage("Error", "${ExampleFixtureServer.baseUrl}/smoke/error.html"),
        FixturePage("Policy", "${ExampleFixtureServer.baseUrl}/smoke/policy.html")
      )

    private val CONTROL_FIXTURES =
      listOf(
        FixturePage("Controls", "${ExampleFixtureServer.baseUrl}/controls/index.html"),
        FixturePage("Dialogs", "${ExampleFixtureServer.baseUrl}/controls/dialogs.html"),
        FixturePage("Text input", "${ExampleFixtureServer.baseUrl}/controls/ime-form.html"),
        FixturePage("Select", "${ExampleFixtureServer.baseUrl}/controls/select-elements.html"),
        FixturePage("Pickers", "${ExampleFixtureServer.baseUrl}/controls/pickers.html"),
        FixturePage("File", "${ExampleFixtureServer.baseUrl}/controls/file-input.html"),
        FixturePage("Permissions", "${ExampleFixtureServer.baseUrl}/controls/permissions.html"),
        FixturePage("Context", "${ExampleFixtureServer.baseUrl}/controls/context-menu-demo.html")
      )

    private const val FILE_PICKER_REQUEST_CODE = 0x5346
    private const val ANDROID_PERMISSION_REQUEST_CODE = 0x5350

    private val FILE_PICKER_EXTENSION_MIME_TYPES =
      mapOf(
        "md" to "text/markdown",
        "markdown" to "text/markdown"
      )

    private val DISPLAYED_EVENT_NAMES =
      setOf(
        "navigationRequested",
        "urlChanged",
        "loadStatusChanged",
        "pageTitleChanged",
        "historyChanged",
        "simpleDialogRequested",
        "simpleDialogDismissed",
        "inputMethodRequested",
        "inputMethodDismissed",
        "selectElementRequested",
        "selectElementDismissed",
        "filePickerRequested",
        "filePickerDismissed",
        "permissionRequested",
        "contextMenuRequested",
        "contextMenuDismissed",
        "focusChanged",
        "error"
      )
  }
}

private data class FixturePage(
  val label: String,
  val url: String
)
