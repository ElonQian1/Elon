package com.elon.app.chatgptweb

import org.json.JSONObject

/** Read-only admission before the one-use server authorization; never retries a send. */
internal class GroupWebAiSendPreparation(
    private val probe: (String) -> Unit,
    private val schedule: (Long, () -> Unit) -> Unit,
    private val onReady: () -> Unit,
    private val observe: (Map<String, Any?>) -> Unit,
    private val nextId: () -> String = GroupWebAiCommandIds::next,
) {
    private var started = false
    private var closed = false
    private var attempts = 0
    private var pending: String? = null

    fun start() {
        if (started || closed) return
        started = true
        read()
    }

    private fun read() {
        if (closed || pending != null) return
        val id = nextId()
        attempts++
        pending = id
        schedule(5_500L) {
            if (!closed && pending == id) accept(false, "receipt_timeout", "timeout")
        }
        probe(id)
    }

    fun event(event: ChatGptWebEvent) {
        if (closed || event !is ChatGptWebEvent.CommandResult || event.action != "private_protocol_probe" ||
            event.requestId == null || event.requestId != pending) return
        val safe = ChatGptWebPrivateProtocolEvidence.detail(event.action, event.detail)
        val value = runCatching { JSONObject(safe) }.getOrNull()
            ?.takeIf { it.optString("schema") == "elon.fresh_text_admission.v1" }
        val code = value?.optString("code") ?: "unavailable"
        val stage = value?.optString("stage") ?: "unknown"
        accept(event.ok && code == "ready" && stage == "ready", code, stage)
    }

    private fun accept(ready: Boolean, code: String, stage: String) {
        pending = null
        observe(mapOf("stage" to "send_admission", "code" to code, "admission_stage" to stage, "attempt" to attempts))
        if (ready || attempts > RETRY_DELAYS.size) {
            closed = true
            observe(mapOf("stage" to if (ready) "private_send_prepared" else "official_send_fallback"))
            // Exhaustion retains the existing official sender, still behind the
            // executor's empty temporary-document and identity guards.
            onReady()
        } else schedule(RETRY_DELAYS[attempts - 1]) { read() }
    }

    fun close() { closed = true; pending = null }

    private companion object {
        val RETRY_DELAYS = listOf(500L, 1_000L, 2_000L, 4_000L)
    }
}
