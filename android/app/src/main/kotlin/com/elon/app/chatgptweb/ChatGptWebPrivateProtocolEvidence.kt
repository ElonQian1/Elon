package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

/** Validates structural diagnostics before they enter the native command ledger. */
internal object ChatGptWebPrivateProtocolEvidence {
    val MODES = setOf("start", "read", "stop", "clear", "runtime_assets", "composer_tool_context",
        "stop_runtime_context", "stop_runtime_owner", "directory_refresh", "model_runtime_context", "document_state",
        "library_attachment_policy", "file_download_source")
    private val libraryPolicyCodes = setOf("not_observed", "runtime_unavailable", "validator_unavailable", "document_changed",
        "runtime_changed", "limits_bypassed", "composer_detached", "owner_unavailable", "store_mismatch",
        "scope_mismatch", "model_mismatch", "limits_missing", "limits_invalid", "ready", "attachment_limit", "validator_error")
    private val modelContextCodes = setOf("not_observed", "route_unsupported", "document_unavailable", "identity_unavailable",
        "trigger_detached", "owner_unavailable", "picker_missing", "picker_disabled", "picker_ambiguous",
        "conversation_mismatch", "capture_error", "cooldown", "menu_open", "runtime_not_observed",
        "loading", "runtime_timeout", "runtime_unknown", "runtime_unavailable", "context_changed",
        "catalog_unavailable", "ready")
    private val stopContextCodes = setOf("not_observed", "disabled", "invalid_command", "composer_unavailable",
        "context_unavailable", "request_unavailable", "runtime_not_observed", "preparing", "invoked",
        "document_changed", "context_changed", "request_changed", "runtime_unavailable", "voice_active",
        "generation_not_ready", "already_stopped", "stop_observed", "timeout", "stop_failed",
        "invalid_receipt", "invocation_failed")
    private val toolContextCodes = setOf("not_observed", "ready", "capture_error", "runtime_unavailable",
        "conversation_unavailable", "composer_detached", "owner_unavailable", "props_mismatch",
        "model_unavailable", "cache_unavailable", "eligibility_unavailable", "menu_unavailable",
        "hints_ambiguous", "tool_unavailable")
    private const val ACTION = "private_protocol_probe"
    private const val SCHEMA = "elon.private_protocol_probe.v1"
    private val kinds = setOf("json", "multipart", "stream", "other", "unknown")
    private val states = setOf("skipped", "ready", "pending", "oversize", "invalid", "unavailable", "timeout", "cancelled")
    private val fields = Regex("\\$(?:\\.[A-Za-z][A-Za-z0-9_]{0,39}|\\[\\]){0,4}:(?:null|array|object|string|number|boolean|file)")
    private val paths = Regex("/(?:backend-api|api|ces)(?:/[A-Za-z0-9._{}-]{0,40})+")
    private val rootKeys = setOf("schema", "active", "dropped", "records")
    private val recordKeys = setOf("id", "method", "path", "transport", "status", "requestKind", "responseKind",
        "requestState", "responseState", "requestFields", "responseFields")

    fun detail(action: String, raw: String): String {
        if (action == "share_conversation") return ChatGptWebConversationShareReceipt.detail(raw)
        if (action != ACTION) return raw.take(160)
        if (raw == "protocol_probe_unavailable") return raw
        if (raw.startsWith("model_runtime_context:") && raw.substringAfter(':') in modelContextCodes) return raw
        if (raw.startsWith("composer_tool_context:") && raw.substringAfter(':') in toolContextCodes) return raw
        if (raw.startsWith("library_attachment_policy:") && raw.substringAfter(':') in libraryPolicyCodes) return raw
        if (raw.startsWith("stop_runtime_context:") && raw.substringAfter(':') in stopContextCodes) return raw
        return runCatching { sanitize(raw) }.getOrNull() ?: "invalid_protocol_evidence"
    }

    private fun sanitize(raw: String): String {
        require(raw.length <= 12000)
        val value = JSONObject(raw)
        if (value.opt("schema") == "elon.download_source.v1") {
            require(value.keys().asSequence().toSet() == setOf("schema", "observed", "origin", "path",
                "relative", "whitespace", "credentials", "port", "fragment"))
            for (key in listOf("observed", "relative", "whitespace", "credentials", "port", "fragment")) {
                require(value.opt(key) is Boolean)
            }
            require(value.opt("origin") in setOf("invalid", "same_origin", "non_https", "oaiusercontent", "azure_blob", "other_https"))
            require(value.getString("path").let { it.isEmpty() || Regex("(?:/(?:api|backend-api|files|library|download|content|project-content|estuary|attachment|attachments|\\{id\\})){1,8}/?").matches(it) })
            return value.toString()
        }
        if (value.opt("schema") == "elon.document_state.v1") return documentState(value)
        if (value.opt("schema") == ChatGptWebDirectoryDiagnostic.SCHEMA) {
            return ChatGptWebDirectoryDiagnostic.sanitize(value)
        }
        if (value.opt("schema") == ChatGptWebPrivateRuntimeAssets.SCHEMA) {
            return ChatGptWebPrivateRuntimeAssets.sanitize(value)
        }
        if (value.opt("schema") == "elon.stop_runtime_owner.v1") {
            require(value.keys().asSequence().toSet() ==
                setOf("schema", "cached", "request", "tree", "generation", "mode"))
            for (key in listOf("cached", "tree", "generation")) require(value.opt(key) is Boolean)
            require(value.opt("request") in setOf("missing", "valid", "invalid"))
            require(value.opt("mode") in setOf("unknown", "idle", "streaming", "unread", "voice"))
            return value.toString()
        }
        require(value.keys().asSequence().toSet() == rootKeys)
        require(value.opt("schema") == SCHEMA && value.opt("active") is Boolean)
        require(integer(value, "dropped", 0..999))
        val records = value.getJSONArray("records")
        require(records.length() <= 12)
        val safeRecords = JSONArray()
        for (index in 0 until records.length()) {
            val record = records.getJSONObject(index)
            require(record.keys().asSequence().toSet() == recordKeys)
            require(integer(record, "id", (index + 1)..(index + 1)))
            require(integer(record, "status", 0..599))
            require(record.opt("method") in setOf("GET", "POST", "PATCH", "PUT", "DELETE"))
            require(record.opt("transport") in setOf("fetch", "xhr"))
            val path = record.getString("path")
            require(path.length <= 96 && paths.matches(path))
            for (side in listOf("request", "response")) {
                require(record.opt(side + "Kind") in kinds)
                require(record.opt(side + "State") in states)
                val names = record.getJSONArray(side + "Fields")
                require(names.length() <= 12)
                for (fieldIndex in 0 until names.length()) {
                    val field = names.get(fieldIndex) as? String ?: error("field_type")
                    require(field.length <= 80 && fields.matches(field))
                }
            }
            safeRecords.put(record)
        }
        return JSONObject().put("schema", SCHEMA).put("active", value.getBoolean("active"))
            .put("dropped", value.getInt("dropped")).put("records", safeRecords).toString()
    }

    private fun documentState(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == setOf("schema", "ready", "visibility", "focused", "skin",
            "viewport_width", "viewport_height", "body", "main_count", "form_count", "editable_count",
            "prompt_count", "visible_prompt_count", "prompts", "incomplete"))
        require(value.opt("ready") in setOf("loading", "interactive", "complete", "unknown"))
        require(value.opt("visibility") in setOf("visible", "hidden", "prerender", "unknown"))
        for (key in listOf("focused", "skin", "incomplete")) require(value.opt(key) is Boolean)
        for (key in listOf("viewport_width", "viewport_height")) require(integer(value, key, 0..20000))
        for (key in listOf("main_count", "form_count", "editable_count")) require(integer(value, key, 0..256))
        require(integer(value, "prompt_count", 0..16))
        require(integer(value, "visible_prompt_count", 0..value.getInt("prompt_count")))
        val prompts = value.getJSONArray("prompts")
        require(prompts.length() == minOf(8, value.getInt("prompt_count")))
        fun shape(node: JSONObject) {
            require(node.keys().asSequence().toSet() == setOf("tag", "connected", "editable", "width", "height", "display", "visibility"))
            require(node.opt("tag") in setOf("div", "textarea", "input", "p", "span", "body", "unknown"))
            require(node.opt("display") in setOf("none", "block", "inline", "inline-block", "flex", "grid", "contents", "unknown"))
            require(node.opt("visibility") in setOf("visible", "hidden", "collapse", "unknown"))
            for (key in listOf("connected", "editable")) require(node.opt(key) is Boolean)
            for (key in listOf("width", "height")) require(integer(node, key, 0..20000))
        }
        for (index in 0 until prompts.length()) shape(prompts.getJSONObject(index))
        if (value.opt("body") != JSONObject.NULL) shape(value.getJSONObject("body"))
        return value.toString()
    }

    private fun integer(value: JSONObject, key: String, range: IntRange): Boolean {
        val number = value.opt(key)
        return (number is Int || number is Long) && (number as Number).toLong() in
            range.first.toLong()..range.last.toLong()
    }
}
