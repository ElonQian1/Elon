package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebDirectoryDiagnostic {
    const val SCHEMA = "elon.directory_refresh.v1"
    private val codes = setOf("directory_ready", "directory_partial", "directory_timeout",
        "directory_identity_not_ready", "directory_context_changed", "directory_cursor_stalled",
        "directory_response_invalid", "directory_refresh_failed")

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == setOf("schema", "observed", "durationMs", "identityMs", "reads"))
        require(value.opt("schema") == SCHEMA && value.opt("observed") is Boolean)
        integer(value, "durationMs", 60000)
        integer(value, "identityMs", 60000)
        val reads = value.getJSONArray("reads")
        require(reads.length() <= 2)
        val families = mutableSetOf<String>()
        for (index in 0 until reads.length()) {
            val read = reads.getJSONObject(index)
            require(read.keys().asSequence().toSet() == setOf("family", "pages", "resumedPages", "requests",
                "elapsedMs", "lastRequestMs", "ok", "complete", "truncated", "code"))
            val family = read.getString("family")
            require(family in setOf("conversations", "projects", "project_conversations") && families.add(family))
            require(read.opt("code") in codes)
            for (key in listOf("ok", "complete", "truncated")) require(read.opt(key) is Boolean)
            for (key in listOf("pages", "resumedPages", "requests")) integer(read, key, 10)
            for (key in listOf("elapsedMs", "lastRequestMs")) integer(read, key, 60000)
        }
        return value.toString()
    }

    private fun integer(value: JSONObject, key: String, maximum: Int) {
        val raw = value.opt(key)
        require(raw is Int || raw is Long)
        require((raw as Number).toLong() in 0L..maximum.toLong())
    }
}
