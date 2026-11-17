package com.newkeyboard.khmer

class EngineOutput(
    val commit: String,
    val preedit: String,
    val handled: Boolean,
)

class NativeEngine {
    private var handle = nativeCreate()

    fun onKey(codePoint: Int): EngineOutput = output(nativeOnKey(handle, codePoint))

    fun setMode(mode: Int): EngineOutput = output(nativeSetMode(handle, mode))

    fun commit(): EngineOutput = output(nativeCommit(handle))

    fun select(index: Int): EngineOutput = output(nativeSelect(handle, index))

    fun reset() = nativeReset(handle)

    fun candidates(): List<String> = nativeCandidates(handle)?.toList() ?: emptyList()

    fun close() {
        nativeDestroy(handle)
        handle = 0
    }

    private fun output(values: Array<String>?): EngineOutput {
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

        fun warmUp() = nativeWarmUp()

        @JvmStatic
        private external fun nativeWarmUp()
    }
}
