package com.elon.app.chatgptweb

import org.json.JSONObject

/** Project results are bounded structured data, not a 160-character display message. */
internal object GroupChatGptProjectReceipt {
    const val ACTION = "group_project_request"
    private val scope = Regex("[a-f0-9]{64}")
    private val project = Regex("g-p-[a-fA-F0-9]{32}")
    private val conversation = Regex("[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}")
    private val failures = setOf("project_busy", "project_input_invalid", "project_identity_unavailable",
        "project_identity_changed", "project_auth_required", "project_rate_limited", "project_unavailable",
        "project_adapter_unavailable", "project_not_found", "project_request_rejected", "project_service_unavailable",
        "project_http_failed", "project_request_timeout", "project_response_invalid_json", "project_response_too_large",
        "project_network_failed", "project_runtime_error", "project_response_invalid", "project_permission_required",
        "project_memory_scope_unconfirmed", "project_binding_mismatch", "project_conversation_mismatch",
        "project_binding_ambiguous", "project_create_unresolved", "project_reconciliation_incomplete",
        "project_create_unknown", "project_create_rejected", "project_receipt_invalid")
    private val identityReasons = setOf("document", "runtime", "account", "workspace", "timeout",
        "invalid_json", "response_too_large", "http", "network")
    private const val INVALID = "{\"ok\":false,\"code\":\"project_receipt_invalid\"}"

    fun detail(raw: String): String = runCatching {
        require(raw.length <= 1024)
        val value = JSONObject(raw)
        require(value.opt("ok") is Boolean)
        val code = value.getString("code")
        val allowed = if (value.getBoolean("ok")) {
            require(scope.matches(value.getString("accountScope")))
            when (code) {
                "project_identity_ready" -> setOf("ok", "code", "accountScope")
                "project_ready" -> {
                    require(project.matches(value.getString("projectId")))
                    require(value.opt("memoryScope") == "project_v2")
                    if (value.has("conversationId")) require(conversation.matches(value.getString("conversationId")))
                    setOf("ok", "code", "accountScope", "projectId", "memoryScope", "conversationId")
                }
                else -> error("unexpected_project_success")
            }
        } else {
            require(code in failures)
            if (value.has("notSent")) {
                require(value.opt("notSent") is Boolean)
                require(code != "project_create_unknown" || !value.getBoolean("notSent"))
            }
            if (value.has("identityReason")) require(value.opt("identityReason") in identityReasons)
            setOf("ok", "code", "notSent", "identityReason")
        }
        require(value.keys().asSequence().all { it in allowed })
        value.toString()
    }.getOrDefault(INVALID)
}
