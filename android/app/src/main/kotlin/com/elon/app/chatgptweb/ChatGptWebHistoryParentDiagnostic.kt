package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebHistoryParentDiagnostic {
    const val SCHEMA = "elon.history_parent.v1"
    private val codes = setOf("observed", "mapping_missing", "user_missing", "route_unsupported",
        "document_unavailable", "runtime_unavailable", "identity_unavailable", "owner_changed",
        "timeout", "payload_missing", "conversation_mismatch", "read_failed")
    private val kinds = setOf("null", "empty_root", "client_root", "uuid", "other")
    private val terminals = setOf("not_observed", "null_parent", "missing_parent", "cycle", "invalid_node", "limit")
    private val roles = setOf("none", "root", "system", "developer", "user", "assistant", "tool", "other")
    private val contents = setOf("none", "text", "multimodal_text", "code", "execution_output",
        "user_editable_context", "system_error", "other")

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == setOf("schema", "code", "user_found", "nodes", "terminal"))
        require(value.opt("schema") == SCHEMA && value.opt("code") in codes)
        require(value.opt("user_found") is Boolean && value.opt("terminal") in terminals)
        val nodes = value.getJSONArray("nodes")
        val observed = value.getString("code") == "observed"
        require(value.getBoolean("user_found") == observed)
        require(if (observed) nodes.length() in 1..16 && value.getString("terminal") != "not_observed"
            else nodes.length() == 0 && value.getString("terminal") == "not_observed")
        for (index in 0 until nodes.length()) {
            val node = nodes.getJSONObject(index)
            require(node.keys().asSequence().toSet() == setOf("role", "id_kind", "parent_kind",
                "node_id_matches", "message_id_matches", "hidden", "content_kind", "reciprocal", "children"))
            require(node.opt("role") in roles && node.opt("content_kind") in contents)
            require(node.opt("id_kind") in kinds && node.opt("parent_kind") in kinds)
            for (key in listOf("node_id_matches", "message_id_matches", "hidden", "reciprocal")) {
                require(node.opt(key) is Boolean)
            }
            val count = node.opt("children") as? Number ?: error("count_type")
            require(count.toDouble() == count.toInt().toDouble() && count.toInt() in 0..4096)
            if (index == 0) require(node.getString("role") == "user" && node.getBoolean("node_id_matches") &&
                node.getBoolean("message_id_matches"))
        }
        return value.toString()
    }
}
