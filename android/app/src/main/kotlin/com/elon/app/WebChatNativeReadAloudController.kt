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

internal interface WebChatReadAloudSpeaker {
    fun speak(text: String, onDone: () -> Unit, onError: () -> Unit)
    fun stop()
    fun release()
}

internal fun interface WebChatReadAloudCancellation {
    fun cancel()
}

internal interface WebChatReadAloudScheduler {
    fun post(task: () -> Unit)
    fun postDelayed(delayMs: Long, task: () -> Unit): WebChatReadAloudCancellation
}

internal class WebChatNativeReadAloudController(
    private val speakerFactory: () -> WebChatReadAloudSpeaker,
    private val scheduler: WebChatReadAloudScheduler,
    private val onFailure: () -> Unit = {},
) {
    constructor(
        context: android.content.Context,
        main: Handler = Handler(Looper.getMainLooper()),
        onFailure: () -> Unit = {},
    ) : this(
        speakerFactory = { AndroidWebChatReadAloudSpeaker(context) },
        scheduler = HandlerWebChatReadAloudScheduler(main),
        onFailure = onFailure,
    )

    private var speaker: WebChatReadAloudSpeaker? = null
    private var generation = 0
    private var chunkGeneration = 0
    private var activeMessageId: String? = null
    private var pending = ArrayDeque<String>()
    private var watchdog: WebChatReadAloudCancellation? = null

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
        chunkGeneration += 1
        activeMessageId = null
        pending.clear()
        clearWatchdog()
        speaker?.stop()
    }

    fun release() {
        stop()
        speaker?.release()
        speaker = null
    }

    private fun speakNext(token: Int) {
        if (token != generation || activeMessageId == null) return
        val next = pending.pollFirst()
        if (next == null) {
            clearWatchdog()
            activeMessageId = null
            return
        }
        val currentSpeaker = speaker ?: runCatching(speakerFactory).getOrElse {
            fail(token, ++chunkGeneration)
            return
        }.also { speaker = it }
        val chunkToken = ++chunkGeneration
        clearWatchdog()
        watchdog = scheduler.postDelayed(CHUNK_TIMEOUT_MS) {
            settle(token, chunkToken, succeeded = false)
        }
        runCatching {
            currentSpeaker.speak(
                next,
                onDone = { scheduler.post { settle(token, chunkToken, succeeded = true) } },
                onError = { scheduler.post { settle(token, chunkToken, succeeded = false) } },
            )
        }.onFailure {
            scheduler.post { settle(token, chunkToken, succeeded = false) }
        }
    }

    private fun settle(token: Int, chunkToken: Int, succeeded: Boolean) {
        if (token != generation || chunkToken != chunkGeneration || activeMessageId == null) return
        clearWatchdog()
        if (succeeded) speakNext(token) else fail(token, chunkToken)
    }

    private fun fail(token: Int, chunkToken: Int) {
        if (token != generation || chunkToken != chunkGeneration) return
        generation += 1
        chunkGeneration += 1
        activeMessageId = null
        pending.clear()
        clearWatchdog()
        val failedSpeaker = speaker
        speaker = null
        failedSpeaker?.release()
        onFailure()
    }

    private fun clearWatchdog() {
        watchdog?.cancel()
        watchdog = null
    }

    private companion object {
        const val CHUNK_TIMEOUT_MS = 180_000L
    }
}

private class AndroidWebChatReadAloudSpeaker(context: android.content.Context) : WebChatReadAloudSpeaker {
    private val delegate = VoiceSpeaker(context, respectUserToggle = false)

    override fun speak(text: String, onDone: () -> Unit, onError: () -> Unit) {
        delegate.speak(text, onDone = onDone, onError = onError)
    }

    override fun stop() = delegate.stop()

    override fun release() = delegate.release()
}

private class HandlerWebChatReadAloudScheduler(
    private val handler: Handler,
) : WebChatReadAloudScheduler {
    override fun post(task: () -> Unit) {
        handler.post(task)
    }

    override fun postDelayed(delayMs: Long, task: () -> Unit): WebChatReadAloudCancellation {
        val runnable = Runnable(task)
        handler.postDelayed(runnable, delayMs)
        return WebChatReadAloudCancellation { handler.removeCallbacks(runnable) }
    }
}
