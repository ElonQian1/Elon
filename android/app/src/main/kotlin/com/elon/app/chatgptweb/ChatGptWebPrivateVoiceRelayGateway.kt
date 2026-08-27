package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.BuildConfig
import java.util.UUID

internal sealed interface ChatGptWebPrivateVoiceRelayResult {
    data class Success(
        val answer: ChatGptWebPrivateVoiceAnswer,
    ) : ChatGptWebPrivateVoiceRelayResult

    data class Failure(
        val code: String,
    ) : ChatGptWebPrivateVoiceRelayResult
}

internal class ChatGptWebPrivateVoiceRelayGateway(
    private val webView: () -> WebView?,
    private val schedule: (Runnable, Long) -> Unit,
    private val nowMs: () -> Long = System::currentTimeMillis,
    private val requestId: () -> String = {
        "relay_" + UUID.randomUUID().toString().replace("-", "").take(16)
    },
) {
    private var generation = 0L
    private var activeRequestId: String? = null
    private var completion: ((ChatGptWebPrivateVoiceRelayResult) -> Unit)? = null
    private var deadlineMs = 0L

    fun readBootstrap(
        onComplete: (ChatGptWebPrivateVoiceBootstrap) -> Unit,
    ): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED) {
            onComplete(ChatGptWebPrivateVoiceBootstrap.Unavailable("disabled"))
            return false
        }
        val view = webView()
        if (view == null) {
            onComplete(ChatGptWebPrivateVoiceBootstrap.Unavailable("unavailable"))
            return false
        }
        view.evaluateJavascript(ChatGptWebPrivateVoiceRelayContract.bootstrapScript()) { raw ->
            onComplete(ChatGptWebPrivateVoiceRelayContract.parseBootstrap(raw))
        }
        return true
    }

    fun exchange(
        offer: String,
        onComplete: (ChatGptWebPrivateVoiceRelayResult) -> Unit,
    ): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED) {
            onComplete(ChatGptWebPrivateVoiceRelayResult.Failure("disabled"))
            return false
        }
        if (activeRequestId != null) {
            onComplete(ChatGptWebPrivateVoiceRelayResult.Failure("busy"))
            return false
        }
        val view = webView()
        val id = requestId()
        val script = ChatGptWebPrivateVoiceRelayContract.startScript(id, offer)
        if (view == null || script == null) {
            onComplete(ChatGptWebPrivateVoiceRelayResult.Failure("unavailable"))
            return false
        }
        generation += 1
        val token = generation
        activeRequestId = id
        completion = onComplete
        deadlineMs = nowMs() + EXCHANGE_TIMEOUT_MS
        view.evaluateJavascript(script, null)
        schedule(Runnable { poll(token, id) }, FIRST_POLL_DELAY_MS)
        return true
    }

    fun cancel() {
        if (activeRequestId == null) return
        finish(ChatGptWebPrivateVoiceRelayResult.Failure("cancelled"))
    }

    private fun poll(token: Long, id: String) {
        if (token != generation || activeRequestId != id) return
        if (nowMs() >= deadlineMs) {
            finish(ChatGptWebPrivateVoiceRelayResult.Failure("timeout"))
            return
        }
        val view = webView()
        val script = ChatGptWebPrivateVoiceRelayContract.pollScript(id)
        if (view == null || script == null) {
            finish(ChatGptWebPrivateVoiceRelayResult.Failure("unavailable"))
            return
        }
        view.evaluateJavascript(script) { raw ->
            if (token != generation || activeRequestId != id) return@evaluateJavascript
            when (val result = ChatGptWebPrivateVoiceRelayContract.parsePoll(raw)) {
                ChatGptWebPrivateVoiceRelayPoll.Pending ->
                    schedule(Runnable { poll(token, id) }, POLL_DELAY_MS)
                is ChatGptWebPrivateVoiceRelayPoll.Ready ->
                    finish(ChatGptWebPrivateVoiceRelayResult.Success(result.answer))
                is ChatGptWebPrivateVoiceRelayPoll.Failed ->
                    finish(ChatGptWebPrivateVoiceRelayResult.Failure(result.code))
            }
        }
    }

    private fun finish(result: ChatGptWebPrivateVoiceRelayResult) {
        val callback = completion
        generation += 1
        activeRequestId = null
        completion = null
        deadlineMs = 0L
        callback?.invoke(result)
    }

    private companion object {
        const val FIRST_POLL_DELAY_MS = 80L
        const val POLL_DELAY_MS = 120L
        const val EXCHANGE_TIMEOUT_MS = 16_500L
    }
}
