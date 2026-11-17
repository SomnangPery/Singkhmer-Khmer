package com.newkeyboard.khmer

/** The three ways of typing. The mode key goes through them in this order. */
enum class InputMode(val label: String) {
    /** A Khmer layout. Keys go through the engine's Khmer mode. */
    KHMER("ខ្មែរ"),

    /** Latin letters that spell Khmer words. Keys go through the engine's romanized mode. */
    SINGKHMER("Singkhmer"),

    /** Plain Latin letters, written straight into the app. */
    ENGLISH("English"),
}

/** Which set of keys is showing. */
enum class Layer { LETTERS, SYMBOLS, MORE_SYMBOLS }

enum class ShiftState { OFF, ON, LOCKED }

enum class KeyAction { TEXT, SPACE, SHIFT, BACKSPACE, ENTER, MODE, LETTERS, SYMBOLS, MORE_SYMBOLS }

class Key(
    /** What is drawn on the key. Keys with an icon have none. */
    val label: String,
    val action: KeyAction = KeyAction.TEXT,
    /** What the key types. */
    val text: String = label,
    /** What the key types when held, shown small in its corner. */
    val longPress: String? = null,
    /** Width, where a normal key is 1. */
    val width: Float = 1f,
)

/** Rows of keys. A row narrower than [columns] is centred. */
class KeyboardLayout(val columns: Float, val rows: List<List<Key>>)

/**
 * The key layouts. Every layout has five rows, so the keyboard keeps its
 * height when the layout changes.
 */
object Layouts {

    fun get(mode: InputMode, layer: Layer, shifted: Boolean): KeyboardLayout = when (layer) {
        Layer.LETTERS -> if (mode == InputMode.KHMER) khmer(shifted) else latin(mode, shifted)
        Layer.SYMBOLS -> symbols(mode)
        Layer.MORE_SYMBOLS -> moreSymbols(mode)
    }

    /** Keys from labels separated by spaces. [longPress] has one entry per key. */
    private fun keys(labels: String, longPress: String? = null, width: Float = 1f): List<Key> {
        val held = longPress?.split(' ')
        return labels.split(' ').mapIndexed { index, label ->
            Key(label, longPress = held?.get(index), width = width)
        }
    }

    private fun latin(mode: InputMode, shifted: Boolean): KeyboardLayout {
        val top = "q w e r t y u i o p"
        val middle = "a s d f g h j k l"
        val bottom = "z x c v b n m"
        fun cased(letters: String) = if (shifted) letters.uppercase() else letters
        return KeyboardLayout(
            columns = 10f,
            rows = listOf(
                keys("1 2 3 4 5 6 7 8 9 0"),
                keys(cased(top)),
                keys(cased(middle)),
                listOf(Key("", KeyAction.SHIFT, width = 1.5f)) +
                    keys(cased(bottom)) +
                    Key("", KeyAction.BACKSPACE, width = 1.5f),
                latinBottomRow(mode, Key("?123", KeyAction.SYMBOLS, width = 1.5f)),
            ),
        )
    }

    private fun latinBottomRow(mode: InputMode, layerKey: Key) = listOf(
        layerKey,
        Key("", KeyAction.MODE, width = 1.2f),
        Key(","),
        Key(mode.label, KeyAction.SPACE, text = " ", width = 3.8f),
        Key("."),
        Key("", KeyAction.ENTER, width = 1.5f),
    )

    /**
     * The standard Khmer (NiDA) layout, without its rightmost keys: those are
     * on the "more symbols" page. Holding a key gives its shifted character.
     */
    private fun khmer(shifted: Boolean): KeyboardLayout {
        val rows = if (!shifted) {
            listOf(
                keys("១ ២ ៣ ៤ ៥ ៦ ៧ ៨ ៩ ០", longPress = "! ៗ \" ៛ % ៍ ័ ៏ ( )", width = 1.1f),
                keys("ឆ ឹ េ រ ត យ ុ ិ ោ ផ ៀ", longPress = "ឈ ឺ ែ ឬ ទ ួ ូ ី ៅ ភ ឿ"),
                keys("ា ស ដ ថ ង ហ ្ ក ល ើ ់", longPress = "ាំ ៃ ឌ ធ អ ះ ញ គ ឡ ោះ ៉"),
                keys("ឋ ខ ច វ ប ន ម ុំ ៊", longPress = "ឍ ឃ ជ េះ ព ណ ំ ុះ ?"),
            )
        } else {
            listOf(
                keys("! ៗ \" ៛ % ៍ ័ ៏ ( )", width = 1.1f),
                keys("ឈ ឺ ែ ឬ ទ ួ ូ ី ៅ ភ ឿ"),
                keys("ាំ ៃ ឌ ធ អ ះ ញ គ ឡ ោះ ៉"),
                keys("ឍ ឃ ជ េះ ព ណ ំ ុះ ?"),
            )
        }
        return KeyboardLayout(
            columns = 11f,
            rows = listOf(
                rows[0],
                rows[1],
                rows[2],
                listOf(Key("", KeyAction.SHIFT)) + rows[3] + Key("", KeyAction.BACKSPACE),
                listOf(
                    Key("?123", KeyAction.SYMBOLS, width = 1.6f),
                    Key("", KeyAction.MODE, width = 1.3f),
                    Key(InputMode.KHMER.label, KeyAction.SPACE, text = " ", width = 4.5f),
                    if (shifted) Key("៕") else Key("។", longPress = "៕"),
                    Key("", KeyAction.ENTER, width = 2.4f),
                ),
            ),
        )
    }

    private fun lettersKey(mode: InputMode) =
        Key(if (mode == InputMode.KHMER) "កខគ" else "ABC", KeyAction.LETTERS, width = 1.5f)

    private fun symbols(mode: InputMode) = KeyboardLayout(
        columns = 10f,
        rows = listOf(
            keys("1 2 3 4 5 6 7 8 9 0"),
            keys("@ # $ ៛ % & - + ( )"),
            keys("* \" ' : ; ! ? / = _"),
            listOf(Key("ឥឧ", KeyAction.MORE_SYMBOLS, width = 1.5f)) +
                keys("« » ៖ ៗ ៕ \\ ~") +
                Key("", KeyAction.BACKSPACE, width = 1.5f),
            latinBottomRow(mode, lettersKey(mode)),
        ),
    )

    /** Khmer digits, the independent vowels, and the rarer signs. */
    private fun moreSymbols(mode: InputMode) = KeyboardLayout(
        columns = 10f,
        rows = listOf(
            keys("១ ២ ៣ ៤ ៥ ៦ ៧ ៨ ៩ ០"),
            keys("ឥ ឦ ឧ ឩ ឪ ឫ ឬ ឭ ឮ ឯ"),
            keys("ឰ ឱ ឲ ឳ ៌ ៍ ៎ ៏ ័ ៈ"),
            listOf(Key("?123", KeyAction.SYMBOLS, width = 1.5f)) +
                keys("[ ] { } < > |") +
                Key("", KeyAction.BACKSPACE, width = 1.5f),
            latinBottomRow(mode, lettersKey(mode)),
        ),
    )
}
