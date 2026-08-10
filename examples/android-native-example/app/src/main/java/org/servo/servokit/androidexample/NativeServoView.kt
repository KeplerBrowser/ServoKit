package org.servo.servokit.androidexample

import android.app.AlertDialog
import android.app.DatePickerDialog
import android.app.TimePickerDialog
import android.content.Context
import android.content.DialogInterface
import android.content.res.Resources
import android.graphics.Color
import android.text.Editable
import android.text.InputType
import android.text.Selection
import android.text.SpannableStringBuilder
import android.util.AttributeSet
import android.util.Log
import android.view.Choreographer
import android.view.GestureDetector
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView
import org.servo.servokit.androidhost.IME_COMPOSITION_END
import org.servo.servokit.androidhost.IME_COMPOSITION_UPDATE
import org.servo.servokit.androidhost.KEYBOARD_KEY_BACKSPACE
import org.servo.servokit.androidhost.KEYBOARD_KEY_DELETE
import org.servo.servokit.androidhost.KEYBOARD_KEY_ENTER
import org.servo.servokit.androidhost.ServoColorValue
import org.servo.servokit.androidhost.ServoHostEvent
import org.servo.servokit.androidhost.ServoInputPickerValues
import org.servo.servokit.androidhost.ServoSurfaceLifecycleCoordinator
import org.servo.servokit.androidhost.ServoViewBinding
import org.servo.servokit.androidhost.TOUCH_EVENT_CANCEL
import org.servo.servokit.androidhost.TOUCH_EVENT_DOWN
import org.servo.servokit.androidhost.TOUCH_EVENT_MOVE
import org.servo.servokit.androidhost.TOUCH_EVENT_UP
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.YearMonth

/**
 * Native Android view used by the proof app to host Servo without React Native.
 *
 * This view owns only the proof-app lifecycle glue: a SurfaceView, the shared
 * `crates/servokit-host-android/android` binding, frame updates, and event logging.
 */
class NativeServoView : FrameLayout {
  var onHostEvent: ((NativeHostEvent) -> Unit)? = null

  private val binding = ServoViewBinding(context.applicationContext)
  private val surfaceView = SurfaceView(context)
  private val imeEditable = SpannableStringBuilder()
  private var activeInputMethod: NativeInputMethodRequest? = null
  private var activeInputPickerDialog: AlertDialog? = null
  private var activeInputPickerId: String? = null
  private var contextMenuPointerId: Int? = null
  private var surfaceAttached = false
  private var disposed = false
  private var frameLoopActive = false
  private var frameCallbackPosted = false
  private val frameCallback =
    object : Choreographer.FrameCallback {
      override fun doFrame(frameTimeNanos: Long) {
        frameCallbackPosted = false
        if (disposed || !frameLoopActive) {
          return
        }

        dispatchHostEvents(binding.performUpdates())
        scheduleFrameCallback()
      }
    }
  private val surfaceLifecycleCoordinator =
    ServoSurfaceLifecycleCoordinator(
      onSurfaceAttached = { surface, width, height ->
        surfaceAttached = true
        dispatchHostEvents(binding.attachSurface(surface, width, height, currentDensity()))
      },
      onSurfaceResized = { width, height ->
        dispatchHostEvents(binding.resizeSurface(width, height, currentDensity()))
      },
      onSurfaceDetached = {
        surfaceAttached = false
        dispatchHostEvents(binding.detachSurface())
      },
      onLoadUrl = { url ->
        dispatchHostEvents(binding.loadUrl(url))
        scheduleFrameCallback()
      },
      onRenderingStarted = ::startFrameLoop,
      onRenderingStopped = ::stopFrameLoop
    )
  private val gestureDetector =
    GestureDetector(
      context,
      object : GestureDetector.SimpleOnGestureListener() {
        override fun onDown(e: MotionEvent): Boolean = true

        override fun onLongPress(e: MotionEvent) {
          triggerContextMenu(e)
        }
      }
    )
  private val surfaceCallback =
    object : SurfaceHolder.Callback {
      override fun surfaceCreated(holder: SurfaceHolder) {
        val frame = holder.surfaceFrame
        surfaceLifecycleCoordinator.onSurfaceCreated(
          holder.surface,
          frame.width(),
          frame.height()
        )
      }

      override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        surfaceLifecycleCoordinator.onSurfaceResized(width, height)
      }

      override fun surfaceDestroyed(holder: SurfaceHolder) {
        surfaceLifecycleCoordinator.onSurfaceDestroyed()
      }
    }

  constructor(context: Context) : super(context) {
    initialize()
  }

  constructor(context: Context, attrs: AttributeSet?) : super(context, attrs) {
    initialize()
  }

  constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(
    context,
    attrs,
    defStyleAttr
  ) {
    initialize()
  }

  fun loadUrl(url: String) {
    surfaceLifecycleCoordinator.loadUrl(url)
  }

  fun reload() {
    dispatchBrowserCommand { binding.reload() }
  }

  fun goBack() {
    dispatchBrowserCommand { binding.goBack() }
  }

  fun goForward() {
    dispatchBrowserCommand { binding.goForward() }
  }

  fun resolveNavigationRequest(navigationId: String, allow: Boolean) {
    dispatchBrowserCommand { binding.resolveNavigationRequest(navigationId, allow) }
  }

  fun resolveSimpleDialog(dialogId: String, action: Int, promptValue: String?) {
    dispatchBrowserCommand { binding.resolveSimpleDialog(dialogId, action, promptValue) }
  }

  fun resolveSelectElement(selectElementId: String, selectedOptions: IntArray) {
    dispatchBrowserCommand { binding.resolveSelectElement(selectElementId, selectedOptions) }
  }

  fun resolveContextMenu(contextMenuId: String, action: String) {
    dispatchBrowserCommand { binding.resolveContextMenu(contextMenuId, action) }
  }

  fun dismissContextMenu(contextMenuId: String) {
    dispatchBrowserCommand { binding.dismissContextMenu(contextMenuId) }
  }

  fun resolveFilePicker(filePickerId: String, selectedPaths: Array<String>) {
    dispatchBrowserCommand { binding.resolveFilePicker(filePickerId, selectedPaths) }
  }

  fun dismissFilePicker(filePickerId: String) {
    dispatchBrowserCommand { binding.dismissFilePicker(filePickerId) }
  }

  fun hasPendingPermissionRequest(): Boolean = !disposed && binding.hasPendingPermissionRequest()

  fun getPendingPermissionRequest(): NativePermissionRequest? {
    if (disposed) {
      return null
    }

    return binding.getPendingPermissionRequest()?.let { request ->
      NativePermissionRequest(
        permission = request.permission,
        origin = request.origin
      )
    }
  }

  fun resolvePermission(allow: Boolean) {
    dispatchBrowserCommand { binding.resolvePermission(allow) }
  }

  fun focusWebView() {
    requestFocus()
    dispatchBrowserCommand { binding.focus() }
  }

  fun blurWebView() {
    clearFocus()
    dispatchBrowserCommand { binding.blur() }
  }

  fun dismissInputMethodIfActive(): Boolean {
    if (activeInputMethod == null) {
      return false
    }

    dismissInputMethod()
    return true
  }

  fun destroy() {
    if (disposed) {
      return
    }

    disposed = true
    stopFrameLoop()
    clearActiveInputMethodUi()
    surfaceView.holder.removeCallback(surfaceCallback)
    surfaceLifecycleCoordinator.onViewDisposed()
    binding.dispose()
  }

  override fun onAttachedToWindow() {
    super.onAttachedToWindow()
    if (surfaceAttached) {
      startFrameLoop()
    }
  }

  override fun onDetachedFromWindow() {
    stopFrameLoop()
    super.onDetachedFromWindow()
  }

  override fun performClick(): Boolean {
    super.performClick()
    return true
  }

  override fun onCheckIsTextEditor(): Boolean = activeInputMethod?.isTextInputType() == true

  override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
    val inputMethod = activeInputMethod ?: return null
    if (!inputMethod.isTextInputType()) {
      return null
    }

    outAttrs.inputType = editorInputType(inputMethod)
    outAttrs.imeOptions =
      if (inputMethod.multiline) {
        EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_NONE
      } else {
        EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_DONE
      }
    val selection =
      (inputMethod.insertionPoint ?: inputMethod.text.length).coerceIn(0, imeEditable.length)
    outAttrs.initialSelStart = selection
    outAttrs.initialSelEnd = selection

    return object : BaseInputConnection(this, true) {
      override fun getEditable(): Editable = imeEditable

      override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val committedText = text?.toString().orEmpty()
        val handled = super.commitText(text, newCursorPosition)
        dispatchImeComposition(IME_COMPOSITION_END, committedText)
        return handled
      }

      override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val composingText = text?.toString().orEmpty()
        val handled = super.setComposingText(text, newCursorPosition)
        dispatchImeComposition(IME_COMPOSITION_UPDATE, composingText)
        return handled
      }

      override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
        val handled = super.deleteSurroundingText(beforeLength, afterLength)
        dispatchDeleteKeys(beforeLength, afterLength)
        return handled
      }

      override fun sendKeyEvent(event: KeyEvent): Boolean {
        if (event.action != KeyEvent.ACTION_DOWN) {
          return super.sendKeyEvent(event)
        }

        when (event.keyCode) {
          KeyEvent.KEYCODE_DEL -> {
            dispatchKeyboardKey(KEYBOARD_KEY_BACKSPACE)
            return true
          }
          KeyEvent.KEYCODE_FORWARD_DEL -> {
            dispatchKeyboardKey(KEYBOARD_KEY_DELETE)
            return true
          }
          KeyEvent.KEYCODE_ENTER -> {
            dispatchKeyboardKey(KEYBOARD_KEY_ENTER)
            return true
          }
        }

        return super.sendKeyEvent(event)
      }

      override fun performEditorAction(actionCode: Int): Boolean {
        if (!inputMethod.multiline) {
          dispatchKeyboardKey(KEYBOARD_KEY_ENTER)
          return true
        }
        return super.performEditorAction(actionCode)
      }

      override fun setSelection(start: Int, end: Int): Boolean {
        Selection.setSelection(
          imeEditable,
          start.coerceIn(0, imeEditable.length),
          end.coerceIn(0, imeEditable.length)
        )
        return true
      }
    }
  }

  override fun onKeyPreIme(keyCode: Int, event: KeyEvent): Boolean {
    if (
      keyCode == KeyEvent.KEYCODE_BACK &&
      event.action == KeyEvent.ACTION_UP &&
      activeInputMethod != null
    ) {
      dismissInputMethod()
      return true
    }
    return super.onKeyPreIme(keyCode, event)
  }

  private fun initialize() {
    isFocusable = true
    isFocusableInTouchMode = true
    surfaceView.holder.addCallback(surfaceCallback)
    surfaceView.isClickable = true
    surfaceView.isFocusable = false
    surfaceView.isFocusableInTouchMode = false
    surfaceView.setOnTouchListener { _, event -> handleTouchEvent(event) }
    surfaceView.setOnGenericMotionListener { _, event -> handleGenericMotionEvent(event) }
    addView(surfaceView, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
    setOnFocusChangeListener { _, hasFocus ->
      dispatchHostEvents(if (hasFocus) binding.focus() else binding.blur())
    }
  }

  private fun dispatchBrowserCommand(command: () -> List<ServoHostEvent>) {
    if (disposed) {
      return
    }

    dispatchHostEvents(command())
    scheduleFrameCallback()
  }

  private fun handleTouchEvent(event: MotionEvent): Boolean {
    if (disposed) {
      return false
    }

    if (isSecondaryButtonContextClick(event)) {
      triggerContextMenu(event)
      return true
    }

    gestureDetector.onTouchEvent(event)

    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN,
      MotionEvent.ACTION_POINTER_DOWN -> {
        if (event.actionMasked == MotionEvent.ACTION_DOWN) {
          contextMenuPointerId = null
          if (!hasFocus()) {
            requestFocus()
          }
        }
        dispatchHostTouchEvent(TOUCH_EVENT_DOWN, event.actionIndex, event)
      }
      MotionEvent.ACTION_MOVE -> {
        for (index in 0 until event.pointerCount) {
          if (event.getPointerId(index) == contextMenuPointerId) {
            continue
          }
          dispatchHostTouchEvent(TOUCH_EVENT_MOVE, index, event)
        }
      }
      MotionEvent.ACTION_UP,
      MotionEvent.ACTION_POINTER_UP -> {
        val pointerId = event.getPointerId(event.actionIndex)
        if (pointerId == contextMenuPointerId) {
          contextMenuPointerId = null
          return true
        }

        dispatchHostTouchEvent(TOUCH_EVENT_UP, event.actionIndex, event)
        if (event.actionMasked == MotionEvent.ACTION_UP) {
          performClick()
        }
      }
      MotionEvent.ACTION_CANCEL -> {
        for (index in 0 until event.pointerCount) {
          if (event.getPointerId(index) == contextMenuPointerId) {
            continue
          }
          dispatchHostTouchEvent(TOUCH_EVENT_CANCEL, index, event)
        }
        contextMenuPointerId = null
      }
      else -> return false
    }

    return true
  }

  private fun handleGenericMotionEvent(event: MotionEvent): Boolean {
    if (disposed) {
      return false
    }

    return when {
      isSecondaryButtonContextClick(event) -> {
        triggerContextMenu(event)
        true
      }
      event.actionMasked == MotionEvent.ACTION_BUTTON_RELEASE &&
        event.actionButton == MotionEvent.BUTTON_SECONDARY -> {
        contextMenuPointerId = null
        true
      }
      else -> false
    }
  }

  private fun isSecondaryButtonContextClick(event: MotionEvent): Boolean {
    val isButtonAction =
      event.actionMasked == MotionEvent.ACTION_BUTTON_PRESS || event.actionMasked == MotionEvent.ACTION_DOWN
    val secondaryButtonActive =
      event.actionButton == MotionEvent.BUTTON_SECONDARY ||
        (event.buttonState and MotionEvent.BUTTON_SECONDARY) == MotionEvent.BUTTON_SECONDARY
    return isButtonAction && secondaryButtonActive
  }

  private fun triggerContextMenu(event: MotionEvent) {
    if (disposed || event.pointerCount != 1) {
      return
    }

    val pointerId = event.getPointerId(0)
    if (contextMenuPointerId == pointerId) {
      return
    }

    contextMenuPointerId = pointerId
    dispatchHostTouchEvent(TOUCH_EVENT_CANCEL, 0, event)
    dispatchHostEvents(binding.triggerContextMenu(event.x, event.y))
    scheduleFrameCallback()
  }

  private fun dispatchHostTouchEvent(action: Int, pointerIndex: Int, event: MotionEvent) {
    if (pointerIndex !in 0 until event.pointerCount) {
      return
    }

    dispatchHostEvents(
      binding.dispatchTouchEvent(
        action,
        event.getPointerId(pointerIndex),
        event.getX(pointerIndex),
        event.getY(pointerIndex)
      )
    )
    scheduleFrameCallback()
  }

  private fun handleNativeHostEvent(event: NativeHostEvent) {
    when (event.name) {
      "inputMethodRequested" -> event.inputMethodRequest()?.let(::handleInputMethodRequested)
      "inputMethodDismissed" -> event.inputMethodDismissedId()?.let(::handleInputMethodDismissed)
    }
  }

  private fun handleInputMethodRequested(event: NativeInputMethodRequest) {
    activeInputMethod = event
    if (!hasFocus()) {
      requestFocus()
    }

    if (!event.isTextInputType()) {
      imeEditable.clear()
      inputMethodManager()?.hideSoftInputFromWindow(windowToken, 0)
      inputMethodManager()?.restartInput(this)
      if (activeInputPickerId != event.inputMethodId || activeInputPickerDialog == null) {
        dismissActiveInputPicker()
        showInputPicker(event)
      }
      return
    }

    dismissActiveInputPicker()
    syncEditableFromInputMethod(event)
    inputMethodManager()?.restartInput(this)
    if (event.allowVirtualKeyboard) {
      post {
        if (disposed || activeInputMethod?.inputMethodId != event.inputMethodId) {
          return@post
        }
        inputMethodManager()?.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT)
      }
    }
  }

  private fun handleInputMethodDismissed(inputMethodId: String) {
    if (activeInputMethod?.inputMethodId != inputMethodId) {
      return
    }

    clearActiveInputMethodUi()
  }

  private fun syncEditableFromInputMethod(event: NativeInputMethodRequest) {
    imeEditable.replace(0, imeEditable.length, event.text)
    val selection = (event.insertionPoint ?: event.text.length).coerceIn(0, imeEditable.length)
    Selection.setSelection(imeEditable, selection)
  }

  private fun clearActiveInputMethodUi() {
    dismissActiveInputPicker()
    activeInputMethod = null
    imeEditable.clear()
    inputMethodManager()?.hideSoftInputFromWindow(windowToken, 0)
    inputMethodManager()?.restartInput(this)
  }

  private fun dismissInputMethod() {
    if (disposed) {
      return
    }

    dispatchHostEvents(binding.dismissInputMethod())
    scheduleFrameCallback()
  }

  private fun dispatchImeComposition(state: Int, text: String) {
    if (disposed) {
      return
    }

    dispatchHostEvents(binding.dispatchImeComposition(state, text))
    scheduleFrameCallback()
  }

  private fun dispatchKeyboardKey(key: Int) {
    if (disposed) {
      return
    }

    dispatchHostEvents(binding.dispatchKeyboardKey(key))
    scheduleFrameCallback()
  }

  private fun dispatchDeleteKeys(beforeLength: Int, afterLength: Int) {
    repeat(beforeLength.coerceAtLeast(0)) {
      dispatchKeyboardKey(KEYBOARD_KEY_BACKSPACE)
    }
    repeat(afterLength.coerceAtLeast(0)) {
      dispatchKeyboardKey(KEYBOARD_KEY_DELETE)
    }
  }

  private fun editorInputType(inputMethod: NativeInputMethodRequest): Int {
    var inputType =
      when (inputMethod.type) {
        "email" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
        "number" -> InputType.TYPE_CLASS_NUMBER
        "password" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
        "search" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_WEB_EDIT_TEXT
        "tel" -> InputType.TYPE_CLASS_PHONE
        "text" -> InputType.TYPE_CLASS_TEXT
        "url" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
        else -> InputType.TYPE_CLASS_TEXT
      }
    if (inputMethod.multiline) {
      inputType = inputType or InputType.TYPE_TEXT_FLAG_MULTI_LINE
    }
    return inputType
  }

  private fun inputMethodManager(): InputMethodManager? =
    context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager

  private fun showInputPicker(event: NativeInputMethodRequest) {
    when (event.type) {
      "color" -> showColorPicker(event)
      "date" -> showDatePicker(event)
      "datetime-local" -> showDateTimeLocalPicker(event)
      "month" -> showMonthPicker(event)
      "time" -> showTimePicker(event)
      "week" -> showWeekPicker(event)
      else -> {
        Log.w(TAG, "Picker input method type=${event.type} is not supported; dismissing")
        dismissInputMethod()
      }
    }
  }

  private fun showColorPicker(event: NativeInputMethodRequest) {
    var selectedColor = ServoInputPickerValues.initialColorValue(event.text)
    val padding = dp(16)
    val preview =
      View(context).apply {
        layoutParams = LinearLayout.LayoutParams(
          ViewGroup.LayoutParams.MATCH_PARENT,
          dp(48)
        )
        setBackgroundColor(Color.rgb(selectedColor.red, selectedColor.green, selectedColor.blue))
      }
    val valueLabel =
      TextView(context).apply {
        text = selectedColor.hex()
        setPadding(0, dp(12), 0, dp(8))
      }
    val container =
      LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(padding, padding, padding, 0)
        addView(preview)
        addView(valueLabel)
      }

    fun updateSelectedColor(color: ServoColorValue) {
      selectedColor = color
      valueLabel.text = color.hex()
      preview.setBackgroundColor(Color.rgb(color.red, color.green, color.blue))
    }

    container.addView(createColorSlider("Red", selectedColor.red) { value ->
      updateSelectedColor(selectedColor.copy(red = value))
    })
    container.addView(createColorSlider("Green", selectedColor.green) { value ->
      updateSelectedColor(selectedColor.copy(green = value))
    })
    container.addView(createColorSlider("Blue", selectedColor.blue) { value ->
      updateSelectedColor(selectedColor.copy(blue = value))
    })

    val dialog =
      AlertDialog.Builder(context)
        .setTitle("Pick a color")
        .setView(container)
        .setPositiveButton(android.R.string.ok) { _, _ ->
          commitInputMethodValue(selectedColor.hex())
          dismissInputMethod()
        }
        .setNegativeButton(android.R.string.cancel) { _, _ ->
          dismissInputMethod()
        }
        .create()
    dialog.setOnCancelListener {
      dismissInputMethod()
    }
    showInputPickerDialog(event.inputMethodId, dialog)
  }

  private fun createColorSlider(
    label: String,
    initialValue: Int,
    onChanged: (Int) -> Unit
  ): View =
    LinearLayout(context).apply {
      orientation = LinearLayout.VERTICAL
      val sliderLabel =
        TextView(context).apply {
          text = "$label: $initialValue"
        }
      addView(sliderLabel)
      addView(
        SeekBar(context).apply {
          max = 255
          progress = initialValue
          setOnSeekBarChangeListener(
            object : SeekBar.OnSeekBarChangeListener {
              override fun onProgressChanged(seekBar: SeekBar?, progress: Int, fromUser: Boolean) {
                sliderLabel.text = "$label: $progress"
                onChanged(progress)
              }

              override fun onStartTrackingTouch(seekBar: SeekBar?) = Unit

              override fun onStopTrackingTouch(seekBar: SeekBar?) = Unit
            }
          )
        }
      )
    }

  private fun showDatePicker(event: NativeInputMethodRequest) {
    val initialDate = ServoInputPickerValues.initialDate(event.text, LocalDate.now())
    val dialog =
      createDatePickerDialog("Pick a date", initialDate) { selectedDate ->
        commitInputMethodValue(ServoInputPickerValues.formatDate(selectedDate))
        dismissInputMethod()
      }
    showInputPickerDialog(event.inputMethodId, dialog)
  }

  private fun showDateTimeLocalPicker(event: NativeInputMethodRequest) {
    val initialDateTime = ServoInputPickerValues.initialDateTimeLocal(event.text, LocalDateTime.now())
    val dateDialog =
      createDatePickerDialog("Pick a date", initialDateTime.toLocalDate()) { selectedDate ->
        post {
          if (disposed || activeInputMethod?.inputMethodId != event.inputMethodId) {
            return@post
          }

          val timeDialog =
            createTimePickerDialog("Pick a time", initialDateTime.toLocalTime()) { selectedTime ->
              commitInputMethodValue(
                ServoInputPickerValues.formatDateTimeLocal(LocalDateTime.of(selectedDate, selectedTime))
              )
              dismissInputMethod()
            }
          showInputPickerDialog(event.inputMethodId, timeDialog)
        }
      }
    showInputPickerDialog(event.inputMethodId, dateDialog)
  }

  private fun showMonthPicker(event: NativeInputMethodRequest) {
    val initialMonth = ServoInputPickerValues.initialMonth(event.text, YearMonth.now())
    val dialog =
      createDatePickerDialog(
        title = "Pick a month",
        initialDate = initialMonth.atDay(1),
        hideDay = true
      ) { selectedDate ->
        commitInputMethodValue(
          ServoInputPickerValues.formatMonth(YearMonth.of(selectedDate.year, selectedDate.month))
        )
        dismissInputMethod()
      }
    showInputPickerDialog(event.inputMethodId, dialog)
  }

  private fun showTimePicker(event: NativeInputMethodRequest) {
    val initialTime = ServoInputPickerValues.initialTime(event.text, LocalTime.now())
    val dialog =
      createTimePickerDialog("Pick a time", initialTime) { selectedTime ->
        commitInputMethodValue(ServoInputPickerValues.formatTime(selectedTime))
        dismissInputMethod()
      }
    showInputPickerDialog(event.inputMethodId, dialog)
  }

  private fun showWeekPicker(event: NativeInputMethodRequest) {
    val initialDate = ServoInputPickerValues.initialWeekDate(event.text, LocalDate.now())
    val dialog =
      createDatePickerDialog("Pick a week", initialDate) { selectedDate ->
        commitInputMethodValue(ServoInputPickerValues.formatWeek(selectedDate))
        dismissInputMethod()
      }
    showInputPickerDialog(event.inputMethodId, dialog)
  }

  private fun createDatePickerDialog(
    title: String,
    initialDate: LocalDate,
    hideDay: Boolean = false,
    onDatePicked: (LocalDate) -> Unit
  ): DatePickerDialog =
    DatePickerDialog(
      context,
      { _, year, monthOfYear, dayOfMonth ->
        onDatePicked(LocalDate.of(year, monthOfYear + 1, dayOfMonth))
      },
      initialDate.year,
      initialDate.monthValue - 1,
      initialDate.dayOfMonth
    ).apply {
      setTitle(title)
      setButton(DialogInterface.BUTTON_NEGATIVE, context.getString(android.R.string.cancel)) { _, _ ->
        dismissInputMethod()
      }
      setOnCancelListener {
        dismissInputMethod()
      }
      if (hideDay) {
        val dayFieldId = Resources.getSystem().getIdentifier("day", "id", "android")
        if (dayFieldId != 0) {
          datePicker.findViewById<View>(dayFieldId)?.visibility = View.GONE
        }
      }
    }

  private fun createTimePickerDialog(
    title: String,
    initialTime: LocalTime,
    onTimePicked: (LocalTime) -> Unit
  ): TimePickerDialog =
    TimePickerDialog(
      context,
      { _, hourOfDay, minute ->
        onTimePicked(LocalTime.of(hourOfDay, minute))
      },
      initialTime.hour,
      initialTime.minute,
      true
    ).apply {
      setTitle(title)
      setButton(DialogInterface.BUTTON_NEGATIVE, context.getString(android.R.string.cancel)) { _, _ ->
        dismissInputMethod()
      }
      setOnCancelListener {
        dismissInputMethod()
      }
    }

  private fun showInputPickerDialog(inputMethodId: String, dialog: AlertDialog) {
    activeInputPickerId = inputMethodId
    activeInputPickerDialog = dialog
    dialog.setOnDismissListener {
      if (activeInputPickerDialog === dialog) {
        activeInputPickerDialog = null
        activeInputPickerId = null
      }
    }
    dialog.show()
  }

  private fun dismissActiveInputPicker() {
    val dialog = activeInputPickerDialog ?: return
    activeInputPickerDialog = null
    activeInputPickerId = null
    dialog.setOnDismissListener(null)
    dialog.setOnCancelListener(null)
    dialog.dismiss()
  }

  private fun commitInputMethodValue(value: String) {
    dispatchImeComposition(IME_COMPOSITION_END, value)
  }

  private fun NativeInputMethodRequest.isTextInputType(): Boolean =
    !ServoInputPickerValues.isPickerInputType(type)

  private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

  private fun startFrameLoop() {
    if (disposed || frameLoopActive) {
      return
    }

    frameLoopActive = true
    scheduleFrameCallback()
  }

  private fun stopFrameLoop() {
    frameLoopActive = false
    if (frameCallbackPosted) {
      Choreographer.getInstance().removeFrameCallback(frameCallback)
      frameCallbackPosted = false
    }
  }

  private fun scheduleFrameCallback() {
    if (disposed || !frameLoopActive || frameCallbackPosted) {
      return
    }

    frameCallbackPosted = true
    Choreographer.getInstance().postFrameCallback(frameCallback)
  }

  private fun dispatchHostEvents(events: List<ServoHostEvent>) {
    for (sharedEvent in events) {
      val event = NativeHostEvent.from(sharedEvent)
      handleNativeHostEvent(event)
      Log.i(TAG, "Servokit event: ${event.describe()}")
      onHostEvent?.invoke(event)
    }
  }

  private fun currentDensity(): Float = resources.displayMetrics.density

  companion object {
    private const val TAG = "NativeServoView"
  }
}
