package com.elon.app.chatgptweb

import com.elon.app.DebugTraceStore
import com.elon.app.WebChatProviderId
import java.net.URI

/** Bounded state transitions only: never record prompts, account ids, URLs or response text. */
internal class GroupWebAiDiagnostics(
    private val provider: WebChatProviderId,
    private val record: (String, Map<String, Any?>) -> Unit = DebugTraceStore::record,
) {
    private var previous: Map<String, Any?>? = null
    private var remaining = 24

    fun stage(code: String) = emit(mapOf("stage" to code), code in TERMINAL)

    fun sendPreparation(details: Map<String, Any?>) = emit(details, details["stage"] == "send_failure_context")

    fun sendReceipt(event: ChatGptWebEvent.CommandResult) {
        val receipt = ChatGptWebPrivateTextReceiptPolicy.resolve(event)
        val reason = when {
            event.ok -> "accepted"
            event.detail.startsWith("发送按钮尚未就绪") -> "send_button_not_ready"
            event.detail.startsWith("官方输入框未接受文本") -> "draft_not_accepted"
            event.detail.startsWith("网页草稿已变化") -> "draft_changed"
            event.detail.startsWith("未找到输入框") -> "composer_missing"
            event.detail.startsWith("官方网页未确认发送") -> "send_unconfirmed"
            event.detail.startsWith("official_runtime_v1:rejected:") -> "runtime_rejected"
            receipt.indeterminate -> "send_indeterminate"
            else -> "unknown"
        }
        emit(mapOf("stage" to "send_receipt", "ok" to event.ok, "reason" to reason,
            "authority" to receipt.authority.name.lowercase(), "indeterminate" to receipt.indeterminate), terminal = true)
    }

    fun snapshot(value: ChatGptWebSnapshot, documentUrl: String = value.url,
        ready: Boolean = GroupWebAiSessionPolicy.ready(value, provider, documentUrl)) {
        val route = runCatching { URI(documentUrl) }.getOrNull()
        emit(mapOf(
            "stage" to "snapshot",
            "root_route" to (route?.path in listOf("", "/")),
            "temporary_route" to (route?.rawQuery.orEmpty().split('&').contains("temporary-chat=true")),
            "authenticated" to value.authenticated,
            "login_required" to value.loginRequired,
            "composer_ready" to value.composerReady,
            "private_send_ready" to value.privateSendReady,
            "has_messages" to value.messages.isNotEmpty(),
            "has_draft" to value.draft.isNotBlank(),
            "streaming" to value.streaming,
            "assistant_messages" to value.messages.count { it.role == "assistant" },
            "assistant_characters" to value.messages.filter { it.role == "assistant" }
                .sumOf { it.content.length.toLong() }.coerceAtMost(1_000_000L),
            "private_stream_state" to value.privateStreamState.takeIf {
                it in setOf("", "idle", "streaming", "completed", "interrupted", "failed", "error", "stopped")
            }.orEmpty(),
            "ready" to ready,
        ))
    }

    private fun emit(details: Map<String, Any?>, terminal: Boolean = false) {
        if ((!terminal && remaining <= 0) || details == previous) return
        previous = details
        remaining--
        record("group_web_ai", details + ("provider" to provider.wireValue))
    }

    private companion object {
        val TERMINAL = setOf("prepare_timeout", "response_timeout", "completed", "failed_after_authorize", "failed_before_authorize", "rejected_before_send")
    }
}
