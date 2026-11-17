package com.newkeyboard.khmer

import android.content.Context
import android.inputmethodservice.InputMethodService
import android.os.Build
import android.os.SystemClock
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsets
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.LinearLayout

class KhmerImeService : InputMethodService(), KeyboardView.Listener {

    private lateinit var engine: NativeEngine
    private var keyboardView: KeyboardView? = null
    private var suggestionBar: SuggestionBar? = null

    private var chosenMode = InputMode.SINGKHMER

    private var mode = InputMode.SINGKHMER

    private var plainField = false

    private var layer = Layer.LETTERS
    private var shift = ShiftState.OFF
    private var lastShiftTap = 0L

    private var preedit = ""

    private var lastEdit = 0L

    override fun onCreate() {
        super.onCreate()
        engine = NativeEngine()
        Thread { NativeEngine.warmUp() }.start()
        val saved = getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).getString(KEY_MODE, null)
        chosenMode = InputMode.values().firstOrNull { it.name == saved } ?: InputMode.SINGKHMER
    }

    override fun onDestroy() {
        engine.close()
        super.onDestroy()
    }

    override fun onCreateInputView(): View {
        val palette = Palette.of(this)
        val suggestions = SuggestionBar(this, palette).apply {
            onPick = { index ->
                write(engine.select(index))
                refreshSuggestions()
            }
        }
        val keyboard = KeyboardView(this, palette).apply { listener = this@KhmerImeService }
        suggestionBar = suggestions
        keyboardView = keyboard
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(palette.background)
            addView(suggestions, LinearLayout.LayoutParams(MATCH_PARENT, dp(44f).toInt()))
            addView(keyboard, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        }
        root.setOnApplyWindowInsetsListener { view, insets ->
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val bars = insets.getInsets(
                    WindowInsets.Type.navigationBars() or WindowInsets.Type.displayCutout()
                )
                view.setPadding(bars.left, 0, bars.right, bars.bottom)
            } else {
                @Suppress("DEPRECATION")
                view.setPadding(
                    insets.systemWindowInsetLeft, 0,
                    insets.systemWindowInsetRight, insets.systemWindowInsetBottom,
                )
            }
            insets
        }
        refreshKeyboard()
        refreshSuggestions()
        return root
    }

    override fun onEvaluateFullscreenMode(): Boolean = false

    override fun onStartInput(info: EditorInfo, restarting: Boolean) {
        super.onStartInput(info, restarting)
        engine.reset()
        preedit = ""

        val inputClass = info.inputType and InputType.TYPE_MASK_CLASS
        val variation = info.inputType and InputType.TYPE_MASK_VARIATION
        val numeric = inputClass == InputType.TYPE_CLASS_NUMBER ||
            inputClass == InputType.TYPE_CLASS_PHONE ||
            inputClass == InputType.TYPE_CLASS_DATETIME
        plainField = numeric || info.inputType == InputType.TYPE_NULL ||
            (inputClass == InputType.TYPE_CLASS_TEXT && variation in PLAIN_VARIATIONS)
        mode = if (plainField) InputMode.ENGLISH else chosenMode
        engine.setMode(engineMode(mode))
        if (!restarting) {
            layer = if (numeric) Layer.SYMBOLS else Layer.LETTERS
            shift = ShiftState.OFF
        }
    }

    override fun onStartInputView(info: EditorInfo, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        refreshKeyboard()
        refreshSuggestions()
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        if (preedit.isNotEmpty()) write(engine.commit())
        super.onFinishInputView(finishingInput)
    }

    override fun onFinishInput() {
        super.onFinishInput()
        engine.reset()
        preedit = ""
        refreshSuggestions()
    }

    override fun onKeyTap(key: Key) {
        when (key.action) {
            KeyAction.TEXT, KeyAction.SPACE -> type(key.text)
            KeyAction.BACKSPACE -> backspace()
            KeyAction.ENTER -> enter()
            KeyAction.SHIFT -> tapShift()
            KeyAction.MODE -> switchTo(InputMode.values()[(mode.ordinal + 1) % InputMode.values().size])
            KeyAction.LETTERS -> showLayer(Layer.LETTERS)
            KeyAction.SYMBOLS -> showLayer(Layer.SYMBOLS)
            KeyAction.MORE_SYMBOLS -> showLayer(Layer.MORE_SYMBOLS)
        }
    }

    override fun onKeyLongPress(key: Key): Boolean {
        if (key.action == KeyAction.MODE) {
            getSystemService(InputMethodManager::class.java)?.showInputMethodPicker()
            return true
        }
        val text = key.longPress ?: return false
        type(text)
        return true
    }

    private fun type(text: String) {
        if (mode == InputMode.ENGLISH) {
            currentInputConnection?.commitText(text, 1)
            lastEdit = SystemClock.uptimeMillis()
        } else {
            val commit = StringBuilder()
            var typing = preedit
            text.codePoints().forEach { codePoint ->
                val output = engine.onKey(codePoint)
                commit.append(output.commit)
                typing = output.preedit
            }
            write(commit.toString(), typing)
        }
        if (shift == ShiftState.ON) {
            shift = ShiftState.OFF
            refreshKeyboard()
        }
        refreshSuggestions()
    }

    private fun backspace() {
        if (mode != InputMode.ENGLISH) {
            val output = engine.onKey(NativeEngine.BACKSPACE)
            if (output.handled) {
                write(output)
                refreshSuggestions()
                return
            }
        }
        sendDownUpKeyEvents(KeyEvent.KEYCODE_DEL)
        lastEdit = SystemClock.uptimeMillis()
    }

    private fun enter() {
        if (mode != InputMode.ENGLISH) {
            write(engine.commit())
            engine.reset()
            refreshSuggestions()
        }
        sendKeyChar('\n')
        lastEdit = SystemClock.uptimeMillis()
    }

    private fun tapShift() {
        val now = SystemClock.uptimeMillis()
        shift = when {
            shift == ShiftState.LOCKED -> ShiftState.OFF
            now - lastShiftTap < DOUBLE_TAP_MS -> ShiftState.LOCKED
            shift == ShiftState.ON -> ShiftState.OFF
            else -> ShiftState.ON
        }
        lastShiftTap = now
        refreshKeyboard()
    }

    private fun showLayer(newLayer: Layer) {
        layer = newLayer
        shift = ShiftState.OFF
        refreshKeyboard()
    }

    private fun switchTo(newMode: InputMode) {
        write(if (newMode == InputMode.ENGLISH) engine.commit() else engine.setMode(engineMode(newMode)))
        mode = newMode
        if (!plainField) {
            chosenMode = newMode
            getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).edit()
                .putString(KEY_MODE, newMode.name).apply()
        }
        showLayer(Layer.LETTERS)
        refreshSuggestions()
    }

    private fun engineMode(mode: InputMode): Int =
        if (mode == InputMode.KHMER) NativeEngine.MODE_KHMER else NativeEngine.MODE_ROMANIZED

    private fun write(output: EngineOutput) = write(output.commit, output.preedit)

    private fun write(commit: String, typing: String) {
        val connection = currentInputConnection
        if (connection != null) {
            connection.beginBatchEdit()
            if (commit.isNotEmpty()) connection.commitText(commit, 1)
            if (typing.isNotEmpty()) {
                connection.setComposingText(typing, 1)
            } else if (commit.isEmpty() && preedit.isNotEmpty()) {
                connection.commitText("", 1)
            }
            connection.endBatchEdit()
        }
        preedit = typing
        lastEdit = SystemClock.uptimeMillis()
    }

    override fun onUpdateSelection(
        oldSelStart: Int, oldSelEnd: Int,
        newSelStart: Int, newSelEnd: Int,
        candidatesStart: Int, candidatesEnd: Int,
    ) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd)
        if (preedit.isEmpty()) return

        if (candidatesStart < 0) {
            if (SystemClock.uptimeMillis() - lastEdit < OWN_EDIT_MS) return
            currentInputConnection?.finishComposingText()
            engine.reset()
            preedit = ""
            refreshSuggestions()
            return
        }
        if (newSelStart == candidatesEnd && newSelEnd == candidatesEnd) return

        val commit = engine.commit().commit
        engine.reset()
        val connection = currentInputConnection
        if (connection != null) {
            if (commit == preedit) {
                connection.finishComposingText()
            } else {
                val growth = commit.length - (candidatesEnd - candidatesStart)
                fun moved(position: Int) = when {
                    position >= candidatesEnd -> position + growth
                    position > candidatesStart -> candidatesStart + commit.length
                    else -> position
                }
                connection.beginBatchEdit()
                connection.setComposingText(commit, 1)
                connection.finishComposingText()
                connection.setSelection(moved(newSelStart), moved(newSelEnd))
                connection.endBatchEdit()
            }
        }
        preedit = ""
        lastEdit = SystemClock.uptimeMillis()
        refreshSuggestions()
    }

    private fun refreshKeyboard() {
        val view = keyboardView ?: return
        view.layout = Layouts.get(mode, layer, shift != ShiftState.OFF)
        view.shiftState = shift
        view.enterLabel = enterLabel()
    }

    private fun refreshSuggestions() {
        val suggestions = if (mode == InputMode.SINGKHMER) engine.candidates() else emptyList()
        suggestionBar?.show(suggestions)
    }

    private fun enterLabel(): String? {
        val options = currentInputEditorInfo?.imeOptions ?: return null
        if (options and EditorInfo.IME_FLAG_NO_ENTER_ACTION != 0) return null
        return when (options and EditorInfo.IME_MASK_ACTION) {
            EditorInfo.IME_ACTION_GO -> getString(R.string.enter_go)
            EditorInfo.IME_ACTION_SEARCH -> getString(R.string.enter_search)
            EditorInfo.IME_ACTION_SEND -> getString(R.string.enter_send)
            EditorInfo.IME_ACTION_NEXT -> getString(R.string.enter_next)
            EditorInfo.IME_ACTION_DONE -> getString(R.string.enter_done)
            else -> null
        }
    }

    private companion object {
        const val PREFERENCES = "keyboard"
        const val KEY_MODE = "mode"
        const val DOUBLE_TAP_MS = 350L

        const val OWN_EDIT_MS = 200L

        val PLAIN_VARIATIONS = setOf(
            InputType.TYPE_TEXT_VARIATION_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
            InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS,
            InputType.TYPE_TEXT_VARIATION_URI,
        )
    }
}
