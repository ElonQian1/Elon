package com.elon.app.chatgptweb

import org.json.JSONObject

/** Bounded diagnostics for a single page-owned text dispatch, without message or identity fields. */
internal object ChatGptWebFreshTextTrial {
    const val SCHEMA = "elon.fresh_text_trial.v1"

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == setOf("schema", "version", "control", "armed",
            "remaining_ms", "attempts", "pending", "phase", "code", "dispatched", "accepted", "reconciled"))
        require(value.opt("version") == 4)
        require(value.opt("control") in setOf("state", "armed", "ended", "busy", "disabled", "disposed",
            "identity_unavailable", "invalid_mode"))
        require(value.opt("phase") in setOf("idle", "preparing", "dispatching", "streaming", "reconciling",
            "uncertain", "rejected", "stopping", "completed"))
        require(value.opt("code") is String && Regex("[a-z_]{0,64}").matches(value.getString("code")))
        for (key in listOf("armed", "pending", "dispatched", "accepted", "reconciled")) {
            require(value.opt(key) is Boolean)
        }
        for ((key, maximum) in listOf("remaining_ms" to 120000L, "attempts" to 32L)) {
            val number = value.opt(key)
            require((number is Int || number is Long) && (number as Number).toLong() in 0..maximum)
        }
        require(value.getBoolean("armed") || value.getLong("remaining_ms") == 0L)
        return value.toString()
    }
}
