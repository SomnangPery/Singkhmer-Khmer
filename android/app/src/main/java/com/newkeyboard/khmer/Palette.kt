package com.newkeyboard.khmer

import android.content.Context
import android.content.res.Configuration
import android.util.TypedValue

/** The keyboard's colours. */
class Palette(
    val background: Int,
    val key: Int,
    /** Shift, Backspace and the other keys that do not type a character. */
    val specialKey: Int,
    val pressedKey: Int,
    val text: Int,
    /** The small corner characters, and the name on the space bar. */
    val hint: Int,
    /** The Enter key, and Shift while it is on. */
    val accent: Int,
    val onAccent: Int,
    val divider: Int,
) {
    companion object {
        /** Light or dark, following the system setting. */
        fun of(context: Context): Palette {
            val night = context.resources.configuration.uiMode and
                Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
            return if (night) {
                Palette(
                    background = 0xFF1B1C1F.toInt(),
                    key = 0xFF3B3D42.toInt(),
                    specialKey = 0xFF2A2C30.toInt(),
                    pressedKey = 0xFF5A5D64.toInt(),
                    text = 0xFFF1F1F3.toInt(),
                    hint = 0xFF9EA1A8.toInt(),
                    accent = 0xFF8AB4F8.toInt(),
                    onAccent = 0xFF10233F.toInt(),
                    divider = 0xFF3B3D42.toInt(),
                )
            } else {
                Palette(
                    background = 0xFFE8EAED.toInt(),
                    key = 0xFFFFFFFF.toInt(),
                    specialKey = 0xFFCDD1D6.toInt(),
                    pressedKey = 0xFFB4B9C0.toInt(),
                    text = 0xFF1B1C1F.toInt(),
                    hint = 0xFF6B6F76.toInt(),
                    accent = 0xFF1565C0.toInt(),
                    onAccent = 0xFFFFFFFF.toInt(),
                    divider = 0xFFCDD1D6.toInt(),
                )
            }
        }
    }
}

/** Density-independent pixels to pixels. */
fun Context.dp(value: Float): Float =
    TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value, resources.displayMetrics)

/** Scaled pixels (text sizes) to pixels. */
fun Context.sp(value: Float): Float =
    TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value, resources.displayMetrics)
