package com.newkeyboard.khmer

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View.MeasureSpec
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * Draws every key layout on the computer, without a phone, and saves each as
 * a picture in the build folder's `keyboard-previews`. Run it with
 * `gradlew testDebugUnitTest` after changing a layout or the drawing code, and
 * look at the pictures.
 */
@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [35], qualifiers = "w411dp-h891dp-420dpi")
class KeyboardPreviewTest {

    private val context: Context = RuntimeEnvironment.getApplication()

    @Test
    fun everyLayoutHasFiveRowsThatFit() {
        for (mode in InputMode.values()) {
            for (layer in Layer.values()) {
                for (shifted in listOf(false, true)) {
                    val layout = Layouts.get(mode, layer, shifted)
                    val name = "$mode $layer shifted=$shifted"
                    // Five rows everywhere, so the keyboard never changes height.
                    assertEquals(name, 5, layout.rows.size)
                    for (row in layout.rows) {
                        val width = row.sumOf { it.width.toDouble() }
                        assertTrue("$name: a row is wider than the keyboard", width <= layout.columns + 0.001)
                    }
                }
            }
        }
    }

    @Test
    fun drawsEveryLayout() {
        val folder = File(System.getProperty("previewDir") ?: "build/keyboard-previews")
        folder.mkdirs()
        val width = context.resources.displayMetrics.widthPixels
        for (night in listOf(false, true)) {
            RuntimeEnvironment.setQualifiers(if (night) "+night" else "+notnight")
            val palette = Palette.of(context)
            for (mode in InputMode.values()) {
                for (layer in Layer.values()) {
                    for (shift in listOf(ShiftState.OFF, ShiftState.LOCKED)) {
                        // Shift only changes the letters.
                        if (layer != Layer.LETTERS && shift != ShiftState.OFF) continue
                        // The symbol pages differ between modes only in two labels.
                        if (layer != Layer.LETTERS && mode == InputMode.SINGKHMER) continue
                        // Dark colours are checked on one layout of each kind.
                        if (night && (layer != Layer.LETTERS || shift != ShiftState.OFF)) continue

                        val view = KeyboardView(context, palette)
                        view.layout = Layouts.get(mode, layer, shift != ShiftState.OFF)
                        view.shiftState = shift
                        view.enterLabel = if (mode == InputMode.ENGLISH) "Search" else null
                        view.measure(
                            MeasureSpec.makeMeasureSpec(width, MeasureSpec.EXACTLY),
                            MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED),
                        )
                        view.layout(0, 0, view.measuredWidth, view.measuredHeight)
                        assertTrue(view.measuredHeight > 0)

                        val bitmap = Bitmap.createBitmap(view.measuredWidth, view.measuredHeight, Bitmap.Config.ARGB_8888)
                        view.draw(Canvas(bitmap))
                        val name = listOf(
                            mode.name, layer.name,
                            if (shift == ShiftState.OFF) "plain" else "shift",
                            if (night) "dark" else "light",
                        ).joinToString("-").lowercase()
                        File(folder, "$name.png").outputStream().use {
                            bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
                        }
                    }
                }
            }
        }
    }
}
