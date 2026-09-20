package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.GroupAiConfiguration
import com.elon.app.WebChatProviderId

/** A separate document, sharing identity but never personal conversation/navigation state. */
internal class GroupWebAiExecutor(
    private val activity: AppCompatActivity,
    private val prompt: String,
    private val authorize: ((Boolean) -> Unit) -> Unit,
    private val onResult: (String) -> Unit,
    private val onFailure: (GroupWebAiFailure) -> Unit,
    private val configuration: GroupAiConfiguration = GroupAiConfiguration(),
) {
    private val handler = Handler(Looper.getMainLooper())
    private val provider = requireNotNull(configuration.engine.providerId)
    private val diagnostics = GroupWebAiDiagnostics(provider)
    private var session: GroupWebAiSession? = null
    private val adapter get() = session?.adapter
    private var modelConfiguration: GroupWebAiModelConfiguration? = null
    private var configured = false
    private var sendPreparation: GroupWebAiSendPreparation? = null
    private var prepared = false
    private var lastSnapshot: ChatGptWebSnapshot? = null
    private var finished = false
    private var dispatching = false
    private var dispatched = false
    private val commandId = GroupWebAiCommandIds.next()
    private val timeout = Runnable {
        diagnostics.stage(if (dispatched) "response_timeout" else "prepare_timeout")
        fail(GroupWebAiFailureReason.TIMEOUT)
    }

    fun start() {
        diagnostics.stage("start")
        handler.postDelayed(timeout, 60_000L)
        try {
            session = GroupWebAiSession(activity, ::event, ::fail, provider, diagnostics::stage)
            session?.start()
        } catch (_: RuntimeException) {
            fail(GroupWebAiFailureReason.PREPARATION)
        }
    }

    private fun event(event: ChatGptWebEvent) {
        if (finished) return
        modelConfiguration?.event(event)
        sendPreparation?.event(event)
        if (finished) return
        when (event) {
            is ChatGptWebEvent.Snapshot -> snapshot(event.value)
            is ChatGptWebEvent.CommandResult ->
                if (event.requestId == commandId) {
                    diagnostics.sendReceipt(event)
                    if (!event.ok) {
                        diagnostics.stage("command_rejected")
                        fail(GroupWebAiFailureReason.SEND)
                    }
                }
            else -> Unit
        }
    }

    private fun snapshot(value: ChatGptWebSnapshot) {
        diagnostics.snapshot(value, session?.documentUrl.orEmpty())
        lastSnapshot = value
        if (!dispatched) {
            if (value.loginRequired) { fail(GroupWebAiFailureReason.LOGIN); return }
            if (dispatching || session?.isReady(value) != true) return
            if (!configured && provider == WebChatProviderId.CHATGPT_WEB) {
                if (modelConfiguration == null) {
                    diagnostics.stage("configure_model")
                    modelConfiguration = GroupWebAiModelConfiguration(GroupAiModelPort.from(requireNotNull(adapter)), configuration.modelPath,
                        onReady = { configured = true; lastSnapshot?.let(::snapshot) },
                        onFailure = { fail(GroupWebAiFailureReason.MODEL) },
                        schedule = { delay, retry -> handler.postDelayed({ if (!finished) retry() }, delay) },
                        observe = diagnostics::stage)
                    modelConfiguration?.start()
                }
                return
            }
            if (!prepared && provider == WebChatProviderId.CHATGPT_WEB) {
                if (sendPreparation == null) {
                    sendPreparation = GroupWebAiSendPreparation(
                        probe = { adapter?.privateProtocolProbe("fresh_text_admission", it) },
                        schedule = { delay, read -> handler.postDelayed({ if (!finished) read() }, delay) },
                        onReady = { prepared = true; lastSnapshot?.let(::snapshot) },
                        observe = diagnostics::sendPreparation,
                    )
                    sendPreparation?.start()
                }
                return
            }
            dispatching = true
            diagnostics.stage("authorize")
            // Once authorization is attempted, a lost response must never trigger paid fallback.
            authorize { permitted ->
                if (finished) return@authorize
                if (!permitted) { fail(); return@authorize }
                if (lastSnapshot?.let { session?.isReady(it) } != true) { fail(); return@authorize }
                dispatched = true
                diagnostics.stage("send")
                handler.removeCallbacks(timeout)
                handler.postDelayed(timeout, 180_000L)
                session?.sendPrompt(prompt, commandId)
            }
            return
        }
        val reply = completedReply(value.messages, prompt, value.streaming, value.privateStreamState) ?: return
        diagnostics.stage("completed")
        finish()
        onResult(reply)
    }

    fun cancel() { diagnostics.stage("cancelled"); fail(GroupWebAiFailureReason.CANCELLED) }

    private fun fail(reason: GroupWebAiFailureReason = GroupWebAiFailureReason.PREPARATION) {
        if (finished) return
        val uncertain = dispatching || dispatched
        diagnostics.stage(if (uncertain) "failed_after_authorize" else "failed_before_authorize")
        finish()
        onFailure(GroupWebAiFailure(uncertain, reason))
    }

    private fun finish() {
        finished = true
        handler.removeCallbacksAndMessages(null)
        sendPreparation?.close()
        session?.close()
        session = null
    }

    companion object {
        fun completedReply(messages: List<ChatGptWebMessage>, prompt: String, streaming: Boolean, streamState: String): String? {
            val expected = normalizePrompt(prompt)
            val source = messages.indexOfLast { it.role == "user" && normalizePrompt(it.content) == expected }
            if (source < 0 || streaming) return null
            val reply = messages.drop(source + 1).lastOrNull {
                it.role == "assistant" && it.content.isNotBlank()
            } ?: return null
            if (streamState != "completed" && reply.state !in listOf("complete", "completed", "finished_successfully")) return null
            return reply.content
        }
        private fun normalizePrompt(value: String) = value.trim().replace(Regex("\\s+"), " ")
    }
}
