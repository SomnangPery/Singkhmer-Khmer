package com.newkeyboard.khmer

/** What the keyboard should do after a call into the engine. */
class EngineOutput(
    /** Finished text to write into the app. */
    val commit: String,
    /** The text still being typed. Replaces the composing text shown before. */
    val preedit: String,
    /** False only when the engine had nothing to do with the key. */
    val handled: Boolean,
)

/**
 * The Rust engine (crates/khmer-core), reached through crates/khmer-android.
 * All Khmer logic is there: this class only passes values across.
 *
 * Not thread-safe: use one engine from one thread. Call [close] when done.
 */
class NativeEngine {
    private var handle = nativeCreate()

    /** One key press: a Unicode code point, or [BACKSPACE]. */
    fun onKey(codePoint: Int): EngineOutput = output(nativeOnKey(handle, codePoint))

    /** Switches to [MODE_KHMER] or [MODE_ROMANIZED], committing what was being typed. */
    fun setMode(mode: Int): EngineOutput = output(nativeSetMode(handle, mode))

    /** Finishes whatever is being typed. */
    fun commit(): EngineOutput = output(nativeCommit(handle))

    /** Picks the suggestion at [index] of [candidates]. */
    fun select(index: Int): EngineOutput = output(nativeSelect(handle, index))

    /** Throws away whatever is being typed, and the memory of earlier words. */
    fun reset() = nativeReset(handle)

    /** Suggestions for the phrase being typed, best first. */
    fun candidates(): List<String> = nativeCandidates(handle)?.toList() ?: emptyList()

    fun close() {
        nativeDestroy(handle)
        handle = 0
    }

    private fun output(values: Array<String>?): EngineOutput {
        // Null means the engine failed. Start it clean rather than carry on
        // from a state nobody can see.
        if (values == null || values.size < 3) {
            nativeReset(handle)
            return EngineOutput("", "", true)
        }
        return EngineOutput(values[0], values[1], values[2] == "1")
    }

    private external fun nativeCreate(): Long
    private external fun nativeDestroy(handle: Long)
    private external fun nativeOnKey(handle: Long, codePoint: Int): Array<String>?
    private external fun nativeSetMode(handle: Long, mode: Int): Array<String>?
    private external fun nativeCommit(handle: Long): Array<String>?
    private external fun nativeSelect(handle: Long, index: Int): Array<String>?
    private external fun nativeReset(handle: Long)
    private external fun nativeCandidates(handle: Long): Array<String>?

    companion object {
        const val BACKSPACE = 8
        const val MODE_KHMER = 0
        const val MODE_ROMANIZED = 1

        init {
            System.loadLibrary("khmer_android")
        }

        /**
         * Loads the dictionary, which takes a moment. Call it once from a
         * background thread so the first key press does not have to wait.
         */
        fun warmUp() = nativeWarmUp()

        @JvmStatic
        private external fun nativeWarmUp()
    }
}
