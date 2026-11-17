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

/**
 * The keyboard. It passes key presses to the Rust engine and does what the
 * engine says: write finished text into the app, and show the text still
 * being typed as composing (underlined) text. It never reorders or converts
 * Khmer text itself.
 */
class KhmerImeService : InputMethodService(), KeyboardView.Listener {

    private lateinit var engine: NativeEngine
    private var keyboardView: KeyboardView? = null
    private var suggestionBar: SuggestionBar? = null

    /** The mode picked with the mode key. Remembered between sessions. */
    private var chosenMode = InputMode.SINGKHMER

    /** The mode in use in this text field. */
    private var mode = InputMode.SINGKHMER

    /** True in a field that starts in English whatever the chosen mode: a password, an address. */
    private var plainField = false

    private var layer = Layer.LETTERS
    private var shift = ShiftState.OFF
    private var lastShiftTap = 0L

    /** The composing text this keyboard last put into the app. Empty when there is none. */
    private var preedit = ""

    /** When this keyboard last changed the app's text. */
    private var lastEdit = 0L

    override fun onCreate() {
        super.onCreate()
        engine = NativeEngine()
        // The dictionary takes a moment to load: do it before the first key press.
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
        // Keep the keys clear of the system's navigation bar and of a screen
        // cut-out. The background still reaches the edge of the screen.
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

    /** Stay a keyboard when the phone is held sideways, instead of covering the whole app. */
    override fun onEvaluateFullscreenMode(): Boolean = false

    override fun onStartInput(info: EditorInfo, restarting: Boolean) {
        super.onStartInput(info, restarting)
        // A new text field, or the app changed the text itself: start clean.
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
        // The keyboard is going away: finish the phrase rather than leave it half typed.
        if (preedit.isNotEmpty()) write(engine.commit())
        super.onFinishInputView(finishingInput)
    }

    override fun onFinishInput() {
        super.onFinishInput()
        engine.reset()
        preedit = ""
        refreshSuggestions()
    }

    // ---- key presses ----

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
            // Holding the mode key offers the phone's other keyboards.
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
            // A key can hold more than one character (ុំ is ុ then ំ).
            val commit = StringBuilder()
            var typing = preedit
            text.codePoints().forEach { codePoint ->
                val output = engine.onKey(codePoint)
                commit.append(output.commit)
                typing = output.preedit
            }
            write(commit.toString(), typing)
        }
        // Shift lasts for one key unless it is locked.
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
        // Nothing is being typed: deleting is the app's job.
        sendDownUpKeyEvents(KeyEvent.KEYCODE_DEL)
        lastEdit = SystemClock.uptimeMillis()
    }

    private fun enter() {
        if (mode != InputMode.ENGLISH) {
            write(engine.commit())
            // What follows is a new sentence.
            engine.reset()
            refreshSuggestions()
        }
        // Does what the text field asks of Enter: search, send, or a new line.
        sendKeyChar('\n')
        lastEdit = SystemClock.uptimeMillis()
    }

    private fun tapShift() {
        val now = SystemClock.uptimeMillis()
        shift = when {
            shift == ShiftState.LOCKED -> ShiftState.OFF
            // Two quick taps lock it.
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
        // Whatever was being typed is finished first.
        write(if (newMode == InputMode.ENGLISH) engine.commit() else engine.setMode(engineMode(newMode)))
        mode = newMode
        // In a password field the switch is for that field only.
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

    // ---- writing into the app ----

    private fun write(output: EngineOutput) = write(output.commit, output.preedit)

    /**
     * Writes [commit] into the app as finished text, in place of the composing
     * text, and shows [typing] as the new composing text.
     */
    private fun write(commit: String, typing: String) {
        val connection = currentInputConnection
        if (connection != null) {
            connection.beginBatchEdit()
            if (commit.isNotEmpty()) connection.commitText(commit, 1)
            if (typing.isNotEmpty()) {
                connection.setComposingText(typing, 1)
            } else if (commit.isEmpty() && preedit.isNotEmpty()) {
                // Everything being typed was deleted: remove the composing text.
                connection.commitText("", 1)
            }
            connection.endBatchEdit()
        }
        preedit = typing
        lastEdit = SystemClock.uptimeMillis()
    }

    /**
     * The app reports where the cursor and the composing text are. While a
     * phrase is being typed the cursor is at the end of the composing text;
     * anything else means the user or the app changed something.
     */
    override fun onUpdateSelection(
        oldSelStart: Int, oldSelEnd: Int,
        newSelStart: Int, newSelEnd: Int,
        candidatesStart: Int, candidatesEnd: Int,
    ) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd)
        if (preedit.isEmpty()) return

        if (candidatesStart < 0) {
            // The app has no composing text. Some apps report every step of an
            // edit, and the step between "commit" and "start composing" looks
            // like this too, so a report straight after our own edit is ignored.
            if (SystemClock.uptimeMillis() - lastEdit < OWN_EDIT_MS) return
            // Otherwise the app dropped the text (a chat app after Send).
            currentInputConnection?.finishComposingText()
            engine.reset()
            preedit = ""
            refreshSuggestions()
            return
        }
        if (newSelStart == candidatesEnd && newSelEnd == candidatesEnd) return

        // The cursor was moved away: finish the phrase where it stands.
        val commit = engine.commit().commit
        engine.reset()
        val connection = currentInputConnection
        if (connection != null) {
            if (commit == preedit) {
                connection.finishComposingText()
            } else {
                // The finished text differs from what was showing (a word still
                // in Latin letters becomes Khmer). Swap it in, then put the
                // cursor back where the user put it.
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

    // ---- what is on screen ----

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

    /** The word for what Enter does in this text field, or null for a plain new line. */
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

        /** How long after our own edit a "no composing text" report is taken to be part of it. */
        const val OWN_EDIT_MS = 200L

        /** Text fields where Khmer makes no sense. */
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
