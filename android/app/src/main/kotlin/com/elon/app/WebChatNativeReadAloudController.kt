package com.elon.app

import android.os.Handler
import android.os.Looper
import java.text.BreakIterator
import java.util.ArrayDeque
import java.util.Locale

internal enum class WebChatNativeReadAloudResult {
    STARTED,
    STOPPED,
    EMPTY,
}

internal object WebChatReadAloudChunkPolicy {
    fun chunks(text: String, maxChars: Int = DEFAULT_MAX_CHARS): List<String> {
        val clean = text.trim()
        if (clean.isEmpty() || maxChars <= 0) return emptyList()
        val iterator = BreakIterator.getSentenceInstance(Locale.getDefault()).apply {
            setText(clean)
        }
        val result = mutableListOf<String>()
        val buffer = StringBuilder()
        var start = iterator.first()
        var end = iterator.next()
        while (end != BreakIterator.DONE) {
            appendBounded(result, buffer, clean.substring(start, end).trim(), maxChars)
            start = end
            end = iterator.next()
        }
        flush(result, buffer)
        return result
    }

    private fun appendBounded(
        result: MutableList<String>,
        buffer: StringBuilder,
        sentence: String,
        maxChars: Int,
    ) {
        if (sentence.isBlank()) return
        if (sentence.length > maxChars) {
            flush(result, buffer)
            sentence.chunked(maxChars).map(String::trim).filter(String::isNotEmpty).forEach(result::add)
            return
        }
        val separator = if (buffer.isEmpty()) 0 else 1
        if (buffer.length + separator + sentence.length > maxChars) flush(result, buffer)
        if (buffer.isNotEmpty()) buffer.append(' ')
        buffer.append(sentence)
    }

    private fun flush(result: MutableList<String>, buffer: StringBuilder) {
        if (buffer.isNotEmpty()) result += buffer.toString()
        buffer.clear()
    }

    private const val DEFAULT_MAX_CHARS = 180
}

internal class WebChatNativeReadAloudController(
    context: android.content.Context,
    private val main: Handler = Handler(Looper.getMainLooper()),
) {
    private val speakerDelegate = lazy { VoiceSpeaker(context, respectUserToggle = false) }
    private val speaker by speakerDelegate
    private var generation = 0
    private var activeMessageId: String? = null
    private var pending = ArrayDeque<String>()

    fun isActive(messageId: String): Boolean = activeMessageId == messageId

    fun toggle(messageId: String, text: String): WebChatNativeReadAloudResult {
        if (isActive(messageId)) {
            stop()
            return WebChatNativeReadAloudResult.STOPPED
        }
        val chunks = WebChatReadAloudChunkPolicy.chunks(text)
        if (chunks.isEmpty()) return WebChatNativeReadAloudResult.EMPTY
        stop()
        generation += 1
        val token = generation
        activeMessageId = messageId
        pending = ArrayDeque(chunks)
        speakNext(token)
        return WebChatNativeReadAloudResult.STARTED
    }

    fun stop() {
        generation += 1
        activeMessageId = null
        pending.clear()
        if (speakerDelegate.isInitialized()) speaker.stop()
    }

    fun release() {
        stop()
        if (speakerDelegate.isInitialized()) speaker.release()
    }

    private fun speakNext(token: Int) {
        if (token != generation || activeMessageId == null) return
        val next = pending.pollFirst()
        if (next == null) {
            activeMessageId = null
            return
        }
        speaker.speak(next, onDone = {
            main.post { speakNext(token) }
        })
    }
}
