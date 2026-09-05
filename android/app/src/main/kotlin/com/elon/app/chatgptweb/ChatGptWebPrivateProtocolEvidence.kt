package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

/** Validates structural diagnostics before they enter the native command ledger. */
internal object ChatGptWebPrivateProtocolEvidence {
    val MODES = setOf("start", "read", "stop", "clear")
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
        if (action != ACTION) return raw.take(160)
        if (raw == "protocol_probe_unavailable") return raw
        return runCatching { sanitize(raw) }.getOrNull() ?: "invalid_protocol_evidence"
    }

    private fun sanitize(raw: String): String {
        require(raw.length <= 12000)
        val value = JSONObject(raw)
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

    private fun integer(value: JSONObject, key: String, range: IntRange): Boolean {
        val number = value.opt(key)
        return (number is Int || number is Long) && (number as Number).toLong() in
            range.first.toLong()..range.last.toLong()
    }
}
