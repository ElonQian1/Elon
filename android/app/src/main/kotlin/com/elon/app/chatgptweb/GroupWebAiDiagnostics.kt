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

    fun snapshot(value: ChatGptWebSnapshot, documentUrl: String = value.url) {
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
            "ready" to GroupWebAiSessionPolicy.ready(value, provider, documentUrl),
        ))
    }

    private fun emit(details: Map<String, Any?>, terminal: Boolean = false) {
        if ((!terminal && remaining <= 0) || details == previous) return
        previous = details
        remaining--
        record("group_web_ai", details + ("provider" to provider.wireValue))
    }

    private companion object {
        val TERMINAL = setOf("prepare_timeout", "response_timeout", "completed", "failed_after_authorize", "failed_before_authorize")
    }
}
