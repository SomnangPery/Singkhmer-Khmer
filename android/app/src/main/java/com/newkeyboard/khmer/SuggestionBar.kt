package com.newkeyboard.khmer

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.StateListDrawable
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView

@SuppressLint("ViewConstructor")
class SuggestionBar(context: Context, private val palette: Palette) : HorizontalScrollView(context) {

    var onPick: ((Int) -> Unit)? = null

    private val row = LinearLayout(context)
    private var shown: List<String> = emptyList()

    init {
        setBackgroundColor(palette.background)
        isHorizontalScrollBarEnabled = false
        isFillViewport = true
        row.gravity = Gravity.CENTER
        addView(row, LayoutParams(WRAP_CONTENT, MATCH_PARENT))
    }

    fun show(suggestions: List<String>) {
        if (suggestions == shown) return
        shown = suggestions
        row.removeAllViews()
        suggestions.forEachIndexed { index, suggestion ->
            if (index > 0) row.addView(divider())
            row.addView(item(index, suggestion), LinearLayout.LayoutParams(WRAP_CONTENT, MATCH_PARENT))
        }
        scrollTo(0, 0)
    }

    private fun item(index: Int, suggestion: String) = TextView(context).apply {
        text = suggestion
        gravity = Gravity.CENTER
        maxLines = 1
        minWidth = context.dp(72f).toInt()
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
        setTextColor(palette.text)
        if (index == 0) setTypeface(typeface, Typeface.BOLD)
        val side = context.dp(14f).toInt()
        setPadding(side, 0, side, 0)
        background = StateListDrawable().apply {
            addState(intArrayOf(android.R.attr.state_pressed), ColorDrawable(palette.pressedKey))
        }
        setOnClickListener { onPick?.invoke(index) }
    }

    private fun divider() = View(context).apply {
        setBackgroundColor(palette.divider)
        layoutParams = LinearLayout.LayoutParams(context.dp(1f).toInt(), MATCH_PARENT).apply {
            val gap = context.dp(10f).toInt()
            topMargin = gap
            bottomMargin = gap
        }
    }
}
