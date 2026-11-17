package com.newkeyboard.khmer

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.os.Handler
import android.os.Looper
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import kotlin.math.min

/**
 * Draws the keys of a [KeyboardLayout] and turns touches into key presses.
 * It knows nothing about Khmer or about the app being typed into: every press
 * goes to the [Listener].
 */
@SuppressLint("ViewConstructor")
class KeyboardView(context: Context, private val palette: Palette) : View(context) {

    interface Listener {
        /** A key was pressed and let go. Backspace repeats while it is held. */
        fun onKeyTap(key: Key)

        /** A key is being held. Return true if that did something. */
        fun onKeyLongPress(key: Key): Boolean
    }

    var listener: Listener? = null

    var layout: KeyboardLayout = KeyboardLayout(1f, emptyList())
        set(value) {
            field = value
            placeKeys()
            invalidate()
        }

    var shiftState: ShiftState = ShiftState.OFF
        set(value) {
            field = value
            invalidate()
        }

    /** A word for the Enter key ("Search", "Send"), or null for the arrow. */
    var enterLabel: String? = null
        set(value) {
            field = value
            invalidate()
        }

    private class PlacedKey(val key: Key, val bounds: RectF)

    private var rows: List<List<PlacedKey>> = emptyList()
    private var rowHeight = 0f

    // The key under the finger. A second finger landing finishes the first
    // key at once, so fast two-thumb typing loses no letters.
    private var activePointer = -1
    private var activeKey: Key? = null
    /** True once the press has done its work (a long press, or Backspace). */
    private var consumed = false
    private var repeating = false

    private val timers = Handler(Looper.getMainLooper())
    private val longPress = Runnable {
        val key = activeKey ?: return@Runnable
        if (listener?.onKeyLongPress(key) == true) {
            consumed = true
            performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
        }
    }
    private val repeat = object : Runnable {
        override fun run() {
            val key = activeKey ?: return
            listener?.onKeyTap(key)
            timers.postDelayed(this, REPEAT_EVERY_MS)
        }
    }

    private val sidePadding = context.dp(3f)
    private val topPadding = context.dp(4f)
    private val bottomPadding = context.dp(6f)
    private val gapX = context.dp(2.5f)
    private val gapY = context.dp(4.5f)
    private val corner = context.dp(6f)

    private val keyPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val hintPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        textAlign = Paint.Align.RIGHT
        textSize = context.sp(9f)
        color = palette.hint
    }
    private val iconPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        strokeWidth = context.dp(1.8f)
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }
    private val keyRect = RectF()
    private val iconPath = Path()

    init {
        setBackgroundColor(palette.background)
        isHapticFeedbackEnabled = true
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        // Shorter keys when the screen is short (a phone held sideways).
        val screenHeight = resources.displayMetrics.heightPixels
        val height = min(context.dp(50f) * ROWS, screenHeight * 0.45f)
        setMeasuredDimension(width, (height + topPadding + bottomPadding).toInt())
    }

    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        placeKeys()
    }

    private fun placeKeys() {
        val layoutRows = layout.rows
        if (width == 0 || height == 0 || layoutRows.isEmpty()) {
            rows = emptyList()
            return
        }
        val available = width - 2 * sidePadding
        val unit = available / layout.columns
        rowHeight = (height - topPadding - bottomPadding) / layoutRows.size
        rows = layoutRows.mapIndexed { index, row ->
            val rowWidth = row.sumOf { it.width.toDouble() }.toFloat() * unit
            var x = sidePadding + (available - rowWidth) / 2
            val top = topPadding + index * rowHeight
            row.map { key ->
                val bounds = RectF(x, top, x + key.width * unit, top + rowHeight)
                x = bounds.right
                PlacedKey(key, bounds)
            }
        }
    }

    /** The key nearest to a point: touches in the gaps and margins still count. */
    private fun keyAt(x: Float, y: Float): Key? {
        if (rows.isEmpty()) return null
        val index = ((y - topPadding) / rowHeight).toInt().coerceIn(0, rows.size - 1)
        val row = rows[index]
        return (row.firstOrNull { x < it.bounds.right } ?: row.lastOrNull())?.key
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                release(fire = true)
                val index = event.actionIndex
                press(event.getPointerId(index), keyAt(event.getX(index), event.getY(index)))
            }
            MotionEvent.ACTION_MOVE -> {
                val index = event.findPointerIndex(activePointer)
                // A key that has already done its work stays as it is.
                if (index >= 0 && !consumed && !repeating) {
                    val key = keyAt(event.getX(index), event.getY(index))
                    if (key !== activeKey) slideTo(key)
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                if (event.getPointerId(event.actionIndex) == activePointer) release(fire = true)
            }
            MotionEvent.ACTION_CANCEL -> release(fire = false)
        }
        return true
    }

    private fun press(pointer: Int, key: Key?) {
        activePointer = pointer
        activeKey = key
        consumed = false
        repeating = false
        if (key != null) {
            performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
            if (key.action == KeyAction.BACKSPACE) {
                // Backspace acts at once and repeats while held.
                listener?.onKeyTap(key)
                consumed = true
                repeating = true
                timers.postDelayed(repeat, REPEAT_AFTER_MS)
            } else {
                timers.postDelayed(longPress, LONG_PRESS_MS)
            }
        }
        invalidate()
    }

    /** The finger slid from one key to another without lifting. */
    private fun slideTo(key: Key?) {
        timers.removeCallbacks(longPress)
        activeKey = key
        if (key != null) timers.postDelayed(longPress, LONG_PRESS_MS)
        invalidate()
    }

    private fun release(fire: Boolean) {
        timers.removeCallbacks(longPress)
        timers.removeCallbacks(repeat)
        val key = activeKey
        val wasConsumed = consumed
        activeKey = null
        activePointer = -1
        consumed = false
        repeating = false
        if (key != null) {
            if (fire && !wasConsumed) listener?.onKeyTap(key)
            invalidate()
        }
    }

    override fun onDetachedFromWindow() {
        release(fire = false)
        super.onDetachedFromWindow()
    }

    override fun onDraw(canvas: Canvas) {
        for (row in rows) {
            for (placed in row) drawKey(canvas, placed.key, placed.bounds)
        }
    }

    private fun drawKey(canvas: Canvas, key: Key, bounds: RectF) {
        keyRect.set(bounds.left + gapX, bounds.top + gapY, bounds.right - gapX, bounds.bottom - gapY)
        val accented = key.action == KeyAction.ENTER
        keyPaint.color = when {
            key === activeKey -> palette.pressedKey
            accented -> palette.accent
            key.action == KeyAction.TEXT || key.action == KeyAction.SPACE -> palette.key
            else -> palette.specialKey
        }
        canvas.drawRoundRect(keyRect, corner, corner, keyPaint)

        val ink = if (accented && key !== activeKey) palette.onAccent else palette.text
        when (key.action) {
            KeyAction.SHIFT -> drawShift(canvas)
            KeyAction.BACKSPACE -> drawBackspace(canvas, ink)
            KeyAction.MODE -> drawGlobe(canvas, ink)
            KeyAction.ENTER -> {
                val label = enterLabel
                if (label == null) drawEnter(canvas, ink) else drawLabel(canvas, label, context.sp(14f), ink)
            }
            KeyAction.SPACE -> drawLabel(canvas, key.label, context.sp(13f), palette.hint)
            KeyAction.LETTERS, KeyAction.SYMBOLS, KeyAction.MORE_SYMBOLS ->
                drawLabel(canvas, key.label, context.sp(15f), ink)
            KeyAction.TEXT -> {
                val held = key.longPress
                if (held == null) {
                    drawLabel(canvas, visible(key.label), context.sp(21f), ink)
                } else {
                    // The label sits a little lower and smaller, to leave the
                    // top corner to the character that holding the key gives.
                    drawLabel(canvas, visible(key.label), context.sp(19f), ink, lower = context.dp(4f))
                    canvas.drawText(visible(held), keyRect.right - context.dp(2f), keyRect.top + context.sp(10f), hintPaint)
                }
            }
        }
    }

    /**
     * A Khmer vowel or sign has nothing to sit on when it is alone on a key,
     * so it is drawn on a dotted circle, the usual placeholder.
     */
    private fun visible(label: String): String {
        val first = label.firstOrNull() ?: return label
        val isMark = first in 'ា'..'៓' || first == '៝'
        return if (isMark) "◌$label" else label
    }

    private fun drawLabel(canvas: Canvas, label: String, size: Float, color: Int, lower: Float = 0f) {
        textPaint.color = color
        textPaint.textSize = size
        // Shrink a label that is too wide for its key.
        val room = keyRect.width() - context.dp(4f)
        val width = textPaint.measureText(label)
        if (width > room && width > 0) textPaint.textSize = size * room / width
        val metrics = textPaint.fontMetrics
        val baseline = keyRect.centerY() + lower - (metrics.ascent + metrics.descent) / 2
        canvas.drawText(label, keyRect.centerX(), baseline, textPaint)
    }

    /** Half the size of an icon, which is drawn around the centre of the key. */
    private fun iconHalf(): Float = min(min(keyRect.width(), keyRect.height()) * 0.24f, context.dp(10f))

    /** An upward arrow: outlined when off, filled when on, underlined when locked. */
    private fun drawShift(canvas: Canvas) {
        val h = iconHalf()
        val cx = keyRect.centerX()
        val cy = keyRect.centerY() - h * 0.1f
        iconPath.rewind()
        iconPath.moveTo(cx, cy - h)
        iconPath.lineTo(cx + h, cy)
        iconPath.lineTo(cx + h * 0.45f, cy)
        iconPath.lineTo(cx + h * 0.45f, cy + h * 0.8f)
        iconPath.lineTo(cx - h * 0.45f, cy + h * 0.8f)
        iconPath.lineTo(cx - h * 0.45f, cy)
        iconPath.lineTo(cx - h, cy)
        iconPath.close()
        val on = shiftState != ShiftState.OFF
        iconPaint.color = if (on) palette.accent else palette.text
        iconPaint.style = if (on) Paint.Style.FILL_AND_STROKE else Paint.Style.STROKE
        canvas.drawPath(iconPath, iconPaint)
        if (shiftState == ShiftState.LOCKED) {
            val y = cy + h * 1.25f
            iconPaint.style = Paint.Style.STROKE
            canvas.drawLine(cx - h * 0.45f, y, cx + h * 0.45f, y, iconPaint)
        }
    }

    /** A box pointed to the left, with a cross in it. */
    private fun drawBackspace(canvas: Canvas, color: Int) {
        val h = iconHalf()
        val cx = keyRect.centerX()
        val cy = keyRect.centerY()
        iconPath.rewind()
        iconPath.moveTo(cx - h * 1.25f, cy)
        iconPath.lineTo(cx - h * 0.45f, cy - h * 0.75f)
        iconPath.lineTo(cx + h * 1.1f, cy - h * 0.75f)
        iconPath.lineTo(cx + h * 1.1f, cy + h * 0.75f)
        iconPath.lineTo(cx - h * 0.45f, cy + h * 0.75f)
        iconPath.close()
        iconPaint.color = color
        iconPaint.style = Paint.Style.STROKE
        canvas.drawPath(iconPath, iconPaint)
        val x = cx + h * 0.3f
        val arm = h * 0.3f
        canvas.drawLine(x - arm, cy - arm, x + arm, cy + arm, iconPaint)
        canvas.drawLine(x - arm, cy + arm, x + arm, cy - arm, iconPaint)
    }

    /** An arrow that goes down and then left. */
    private fun drawEnter(canvas: Canvas, color: Int) {
        val h = iconHalf()
        val cx = keyRect.centerX()
        val cy = keyRect.centerY()
        iconPath.rewind()
        iconPath.moveTo(cx + h * 0.9f, cy - h * 0.7f)
        iconPath.lineTo(cx + h * 0.9f, cy + h * 0.25f)
        iconPath.lineTo(cx - h * 0.9f, cy + h * 0.25f)
        iconPath.moveTo(cx - h * 0.4f, cy - h * 0.25f)
        iconPath.lineTo(cx - h * 0.9f, cy + h * 0.25f)
        iconPath.lineTo(cx - h * 0.4f, cy + h * 0.75f)
        iconPaint.color = color
        iconPaint.style = Paint.Style.STROKE
        canvas.drawPath(iconPath, iconPaint)
    }

    /** A globe: the usual sign for "change language". */
    private fun drawGlobe(canvas: Canvas, color: Int) {
        val r = iconHalf() * 0.95f
        val cx = keyRect.centerX()
        val cy = keyRect.centerY()
        iconPaint.color = color
        iconPaint.style = Paint.Style.STROKE
        canvas.drawCircle(cx, cy, r, iconPaint)
        canvas.drawOval(cx - r * 0.45f, cy - r, cx + r * 0.45f, cy + r, iconPaint)
        canvas.drawLine(cx - r, cy, cx + r, cy, iconPaint)
    }

    private companion object {
        const val ROWS = 5
        const val LONG_PRESS_MS = 350L
        const val REPEAT_AFTER_MS = 400L
        const val REPEAT_EVERY_MS = 50L
    }
}
