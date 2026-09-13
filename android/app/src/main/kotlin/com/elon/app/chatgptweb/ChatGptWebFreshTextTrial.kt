package com.elon.app.chatgptweb

import org.json.JSONObject

/** Bounded diagnostics for a single page-owned text dispatch, without message or identity fields. */
internal object ChatGptWebFreshTextTrial {
    const val SCHEMA = "elon.fresh_text_trial.v1"

    fun sanitize(value: JSONObject): String {
        val fields = setOf("schema", "version", "control", "armed",
            "remaining_ms", "attempts", "pending", "phase", "code", "dispatched", "accepted", "reconciled",
            "stream_events", "event_types", "history")
        val version = value.opt("version")
        require(value.opt("schema") == SCHEMA && version in setOf(5, 6, 7))
        // Both v6 shapes shipped. Keep old active writers readable without weakening the v7 contract.
        val hasOperation = version == 7 || version == 6 && value.has("operation")
        val expected = fields + (if (version != 5) setOf("parent_role") else emptySet()) +
            (if (hasOperation) setOf("operation") else emptySet())
        require(value.keys().asSequence().toSet() == expected)
        if (version != 5) require(value.opt("parent_role") in setOf("user", "assistant", "unknown"))
        if (hasOperation) require(value.opt("operation") in setOf("", "send", "regenerate"))
        require(value.opt("control") in setOf("state", "armed", "ended", "busy", "disabled", "disposed",
            "identity_unavailable", "invalid_mode"))
        require(value.opt("phase") in setOf("idle", "preparing", "dispatching", "streaming", "reconciling",
            "uncertain", "rejected", "stopping", "completed"))
        require(value.opt("code") is String && Regex("[a-z_]{0,64}").matches(value.getString("code")))
        for (key in listOf("armed", "pending", "dispatched", "accepted", "reconciled")) {
            require(value.opt(key) is Boolean)
        }
        val maximumAttempts = if (version == 5) 32L else 65535L
        for ((key, maximum) in listOf("remaining_ms" to 120000L, "attempts" to maximumAttempts, "stream_events" to 65535L)) {
            val number = value.opt(key)
            require((number is Int || number is Long) && (number as Number).toLong() in 0..maximum)
        }
        val types = value.getJSONArray("event_types")
        val allowed = setOf("delta_encoding", "message", "input_message", "message_stream_complete",
            "stream_handoff", "resume_conversation_token", "conversation_async_status", "server_ste_metadata",
            "stream-message-start", "stream-message-patch", "stream-message-done", "delta", "other")
        require(types.length() <= allowed.size)
        val seen = mutableSetOf<String>()
        for (index in 0 until types.length()) {
            val type = types.opt(index)
            require(type is String && type in allowed && seen.add(type))
        }
        require(value.opt("history") in setOf("not_observed", "reading", "owner_changed", "payload_missing",
            "conversation_mismatch", "user_missing", "parent_mismatch", "branch_mismatch", "server_active",
            "verified", "not_terminal", "store_not_reconciled", "reconciled"))
        require(value.getBoolean("armed") || value.getLong("remaining_ms") == 0L)
        return value.toString()
    }
}
