package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson

/** Closed metadata only. Never retain an upstream message, URL, account or record. */
internal object BinanceReportDiagnostics {
    fun project(raw: Any?): Map<String, Any?> {
        require(valid(raw))
        val value = raw as Map<*, *>
        return value.entries.associate { (key, item) -> key as String to
            if (key == "http" && item is StrictJson.Number) item.text.toLong() else item }
    }
    fun valid(raw: Any?): Boolean = runCatching {
        val value = raw as? Map<*, *> ?: return false
        require(value.keys == setOf("kind", "stage", "outcome", "error", "http", "business"))
        require(value["kind"] in setOf("none", "history", "orders", "matches", "positions"))
        val endpoints = setOf("history", "detail", "windowOrders", "orders", "matches", "positions")
        require(value["stage"] in endpoints + endpoints.map { "parse_$it" } + setOf("idle", "identity_before", "identity_after"))
        require(value["outcome"] in setOf("idle", "loading", "ready", "failed"))
        require(value["error"] in setOf("none", "unsupported_field", "unsupported_list", "unsupported_object",
            "response_failed", "business_failed", "account_changed", "scope_changed", "restart_pagination",
            "unsupported_total", "transport_or_parse_failed"))
        val http = when (val number = value["http"]) {
            is Int -> number.toString()
            is Long -> number.toString()
            is StrictJson.Number -> number.text
            else -> return false
        }
        require(Regex("0|[1-9][0-9]{0,2}").matches(http) && http.toInt() in 0..599)
        require(value["business"] is String && Regex("none|unknown|[0-9]{6}").matches(value["business"] as String))
        true
    }.getOrDefault(false)
}
