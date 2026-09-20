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
            "stream_events" to value.optInt("stream_events"),
            "event_types" to value.getJSONArray("event_types").let { types ->
                List(types.length()) { types.getString(it) }.joinToString(",")
            },
            "reconciled" to value.optBoolean("reconciled"), "history" to value.optString("history"),
            "reconciliation" to value.optJSONObject("owner")?.optString("reconciliation"),
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
        val CODES = setOf("", "context_changed", "context_invalid", "command_invalid", "invalid_command", "scope_unsupported",
            "runtime_unavailable", "identity_unavailable", "context_unavailable", "attachments_active", "tools_active",
            "conversation_busy", "parent_unavailable", "prepare_unconfirmed", "security_unavailable", "security_invalid",
            "login_required", "draft_changed", "stream_unavailable", "preparation_timeout", "stream_open_timeout",
            "stream_timeout", "stream_item_invalid", "stream_server_error", "stream_handoff_unavailable",
            "stream_topic_owned", "stream_topic_timeout", "stream_item_gap", "stream_item_limit",
            "stream_queue_limit", "stream_subscribe_failed", "stream_owner_changed", "stream_decode_failed",
            "reconciliation_timeout", "cancelled", "request_failed", "recovery_identity_unavailable",
            "recovery_previous_unresolved", "recovery_history_timeout", "recovery_history_unavailable",
            "recovery_storage_unavailable", "recovery_record_invalid", "recovery_record_changed", "recovery_capacity")
    }
}
