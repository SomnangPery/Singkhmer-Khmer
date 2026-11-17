package com.newkeyboard.khmer

import android.content.Context
import android.text.Editable
import android.text.InputType
import android.text.Selection
import android.text.SpannableStringBuilder
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.widget.TextView
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowSystemClock
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35])
class TypingTest {

    private class Field(context: Context) : BaseInputConnection(View(context), true) {
        val content = SpannableStringBuilder()

        override fun getEditable(): Editable = content

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            if (event.action != KeyEvent.ACTION_DOWN) return true
            val cursor = Selection.getSelectionEnd(content)
            when (event.keyCode) {
                KeyEvent.KEYCODE_DEL -> if (cursor > 0) content.delete(cursor - 1, cursor)
                KeyEvent.KEYCODE_ENTER -> content.insert(cursor, "\n")
            }
            return true
        }

        val text: String get() = content.toString()
        val cursor: Int get() = Selection.getSelectionEnd(content)
        val composingStart: Int get() = getComposingSpanStart(content)
        val composingEnd: Int get() = getComposingSpanEnd(content)
        val composing: String
            get() = if (composingStart < 0) "" else content.substring(composingStart, composingEnd)
    }

    private val context: Context = RuntimeEnvironment.getApplication()
    private lateinit var service: KhmerImeService
    private lateinit var field: Field

    @Before
    fun setUp() {
        service = Robolectric.buildService(KhmerImeService::class.java).create().get()
        startField(EditorInfo().apply { inputType = InputType.TYPE_CLASS_TEXT })
    }

    private fun startField(info: EditorInfo, text: String = "") {
        field = Field(context)
        field.content.append(text)
        Selection.setSelection(field.content, text.length)
        ReflectionHelpers.setField(service, "mInputConnection", field)
        ReflectionHelpers.setField(service, "mInputEditorInfo", info)
        service.onStartInput(info, false)
    }

    private fun report() {
        val cursor = Selection.getSelectionStart(field.content)
        service.onUpdateSelection(cursor, cursor, cursor, field.cursor, field.composingStart, field.composingEnd)
    }

    private fun type(keys: String) {
        for (key in keys) {
            service.onKeyTap(if (key == ' ') Key("", KeyAction.SPACE, text = " ") else Key(key.toString()))
            report()
        }
    }

    private fun tap(action: KeyAction) {
        service.onKeyTap(Key("", action))
        report()
    }

    private fun switchToKhmer() {
        tap(KeyAction.MODE)
        tap(KeyAction.MODE)
    }

    @Test
    fun singkhmerPhraseStaysComposingUntilItIsFinished() {
        type("nh jg")
        assertEquals("ខ្ញុំjg", field.text)
        assertEquals("ខ្ញុំjg", field.composing)

        type(" ")
        assertEquals("ខ្ញុំចង់", field.composing)

        type(" ")
        assertEquals("ខ្ញុំចង់ ", field.text)
        assertEquals("", field.composing)
        assertEquals(field.text.length, field.cursor)
    }

    @Test
    fun punctuationFinishesThePhrase() {
        type("soksabay te?")
        assertEquals("សុខសប្បាយទេ?", field.text)
        assertEquals("", field.composing)
    }

    @Test
    fun khmerSyllableTypedOutOfOrderIsStoredInOrder() {
        switchToKhmer()
        type("សី្រ្ត ")
        assertEquals("ស្ត្រី ", field.text)
        assertEquals("", field.composing)
    }

    @Test
    fun aKeyCanHoldTwoCharacters() {
        switchToKhmer()
        type("ក")
        service.onKeyTap(Key("ុំ"))
        report()
        type(" ")
        assertEquals("កុំ ", field.text)
    }

    @Test
    fun backspaceUndoesTypingThenDeletesInTheApp() {
        startField(EditorInfo().apply { inputType = InputType.TYPE_CLASS_TEXT }, text = "ab")
        type("nh")
        assertEquals("abnh", field.text)
        assertEquals("nh", field.composing)

        tap(KeyAction.BACKSPACE)
        assertEquals("abn", field.text)
        tap(KeyAction.BACKSPACE)
        assertEquals("ab", field.text)
        assertEquals("", field.composing)

        tap(KeyAction.BACKSPACE)
        assertEquals("a", field.text)
    }

    @Test
    fun enterFinishesThePhraseAndAddsALine() {
        type("nh jg")
        tap(KeyAction.ENTER)
        assertEquals("ខ្ញុំចង់\n", field.text)
        assertEquals("", field.composing)
    }

    @Test
    fun movingTheCursorFinishesThePhraseAndLeavesTheCursorThere() {
        startField(EditorInfo().apply { inputType = InputType.TYPE_CLASS_TEXT }, text = "abc ")
        type("nh jg")
        assertEquals("abc ខ្ញុំjg", field.text)

        Selection.setSelection(field.content, 1)
        report()
        assertEquals("abc ខ្ញុំចង់", field.text)
        assertEquals("", field.composing)
        assertEquals(1, field.cursor)

        type("nv  ")
        assertEquals("aនៅ bc ខ្ញុំចង់", field.text)
    }

    @Test
    fun aPhraseTheAppThrewAwayDoesNotComeBack() {
        type("nh jg")
        ShadowSystemClock.advanceBy(1, TimeUnit.SECONDS)
        field.content.clear()
        Selection.setSelection(field.content, 0)
        report()

        type("nv  ")
        assertEquals("នៅ ", field.text)
    }

    @Test
    fun aPasswordFieldGetsPlainLatinLetters() {
        startField(
            EditorInfo().apply {
                inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            }
        )
        type("nh jg")
        assertEquals("nh jg", field.text)
        assertEquals("", field.composing)
    }

    @Test
    fun switchingModeFinishesWhatWasBeingTyped() {
        type("nh")
        tap(KeyAction.MODE)
        assertEquals("ខ្ញុំ", field.text)
        assertEquals("", field.composing)
        type("ok")
        assertEquals("ខ្ញុំok", field.text)
    }

    @Test
    fun tappingASuggestionCommitsIt() {
        val root = service.onCreateInputView() as ViewGroup
        type("tv")
        val bar = root.getChildAt(0) as SuggestionBar
        val row = bar.getChildAt(0) as ViewGroup
        val suggestions = (0 until row.childCount).map { row.getChildAt(it) }.filterIsInstance<TextView>()
        assertEquals("ទៅ", suggestions[0].text.toString())
        val other = suggestions[1]
        val chosen = other.text.toString()

        other.performClick()
        report()
        assertEquals(chosen, field.text)
        assertEquals("", field.composing)
    }
}
