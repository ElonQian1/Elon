package com.elon.app.chatgptweb

import org.json.JSONObject

/** One bounded, read-only receipt before destroying a failed group document. */
internal class GroupWebAiFailureInspection(
    private val probe: (String) -> Unit,
    private val schedule: (Long, () -> Unit) -> Unit,
    private val observe: (Map<String, Any?>) -> Unit,
    private val complete: () -> Unit,
) {
    private var pending: String? = null
    private var started = false
    private var closed = false

    fun start() {
        if (started || closed) return
        started = true
        val id = GroupWebAiCommandIds.next()
        pending = id
        schedule(2_000L) { finish(mapOf("code" to "receipt_timeout")) }
        try { probe(id) } catch (_: RuntimeException) { finish(mapOf("code" to "probe_unavailable")) }
    }

    fun event(event: ChatGptWebEvent) {
        if (closed || event !is ChatGptWebEvent.CommandResult || event.action != "private_protocol_probe" ||
            event.requestId == null || event.requestId != pending) return
        val value = runCatching {
            JSONObject(ChatGptWebPrivateProtocolEvidence.detail(event.action, event.detail))
                .takeIf { event.ok && it.optString("schema") == ChatGptWebFreshTextTrial.SCHEMA }
        }.getOrNull()
        if (value == null) { finish(mapOf("code" to "receipt_invalid")); return }
        val code = value.optString("code").takeIf { it in CODES } ?: "unknown"
        finish(mapOf("code" to code, "send_phase" to value.optString("phase"),
            "dispatched" to value.optBoolean("dispatched"), "accepted" to value.optBoolean("accepted"),
            "ownership" to value.optJSONObject("owner")?.optString("ownership")))
    }

    private fun finish(details: Map<String, Any?>) {
        if (closed) return
        closed = true; pending = null
        observe(details + ("stage" to "send_failure_context"))
        complete()
    }

    fun close() { closed = true; pending = null }

    private companion object {
        val CODES = setOf("", "context_changed", "context_invalid", "command_invalid", "scope_unsupported",
            "runtime_unavailable", "identity_unavailable", "context_unavailable", "attachments_active", "tools_active",
            "conversation_busy", "parent_unavailable", "prepare_unconfirmed", "security_unavailable", "security_invalid",
            "login_required", "draft_changed", "stream_unavailable", "preparation_timeout", "stream_open_timeout",
            "stream_timeout", "cancelled", "request_failed", "recovery_identity_unavailable",
            "recovery_previous_unresolved", "recovery_history_timeout", "recovery_history_unavailable",
            "recovery_storage_unavailable", "recovery_record_invalid", "recovery_record_changed", "recovery_capacity")
    }
}
