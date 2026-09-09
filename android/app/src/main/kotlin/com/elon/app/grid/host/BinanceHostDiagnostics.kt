package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson

/** Untrusted page diagnostics cannot create an account, a row or a grant. */
internal class BinanceHostDiagnostics {
    var facts: Map<String, Any?> = emptyMap(); private set
    var observedAt = 0L; private set
    fun clear() { facts = emptyMap(); observedAt = 0 }
    fun accept(event: Map<String, Any?>): Boolean = runCatching {
        val counts = setOf("list_requests", "list_responses", "identity_requests", "identity_responses",
            "detail_requests", "detail_responses", "legacy_list_requests", "script_errors", "script_load_errors", "script_count")
        val fixed = setOf("schema", "token", "last_failure", "failure_kind", "last_http_status", "route",
            "ready_state", "body_text_length", "login_control", "running_control")
        require((event.keys == counts + fixed || event.keys == counts + fixed + "report") && event["schema"] == "yilong.binance_diagnostic.v1")
        if (event.containsKey("report")) require(BinanceReportDiagnostics.valid(event["report"]))
        fun bounded(key: String, maximum: Long): Long {
            val value = event[key]
            val result = when (value) {
                is Long -> value
                is Int -> value.toLong()
                is StrictJson.Number -> {
                    require(Regex("0|[1-9][0-9]*").matches(value.text))
                    value.text.toLong()
                }
                else -> error("COUNT_INVALID")
            }
            require(result in 0..maximum); return result
        }
        counts.forEach { bounded(it, 10000) }; bounded("last_http_status", 599); bounded("body_text_length", 1000000)
        require(event["login_control"] is Boolean && event["running_control"] is Boolean)
        require(event["route"] in setOf("grid", "login", "other"))
        require(event["ready_state"] in setOf("loading", "interactive", "complete", "unknown"))
        require(event["failure_kind"] in setOf("none", "list", "identity", "detail"))
        require(event["last_failure"] in setOf("none", "invalid_field", "invalid_row", "response_failed", "business_failed",
            "identity_unverified", "list_invalid", "duplicate", "account_mismatch", "detail_mismatch",
            "identity_response_failed", "identity_business_failed", "transport_or_parse_failed"))
        facts = event.filterKeys { it != "schema" && it != "token" }.mapValues { (key, value) ->
            when (key) {
                in counts -> bounded(key, 10000)
                "last_http_status" -> bounded(key, 599)
                "body_text_length" -> bounded(key, 1000000)
                "report" -> BinanceReportDiagnostics.project(value)
                else -> value
            }
        }
        observedAt = System.currentTimeMillis()
        true
    }.onFailure { clear() }.getOrDefault(false)
}
