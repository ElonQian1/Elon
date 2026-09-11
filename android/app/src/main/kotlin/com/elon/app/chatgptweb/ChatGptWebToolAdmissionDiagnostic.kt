package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebToolAdmissionDiagnostic {
    const val SCHEMA = "elon.composer_tool_admission.v1"
    private val tools = setOf("web_search", "image_generation", "study", "canvas")
    private val reasons = setOf("raw_missing", "menu_filtered", "ambiguous", "upsell", "different_kind",
        "initial_hidden", "disabled", "different_behavior", "admitted")

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == setOf("schema", "observed", "items"))
        require(value.opt("schema") == SCHEMA && value.opt("observed") is Boolean)
        val rows = value.getJSONArray("items")
        require(rows.length() <= 4 && (value.getBoolean("observed") || rows.length() == 0))
        val seen = mutableSetOf<String>()
        repeat(rows.length()) {
            val row = rows.getJSONObject(it)
            require(row.keys().asSequence().toSet() == setOf("tool", "raw", "menu", "reason"))
            val tool = row.opt("tool") as? String ?: error("invalid_tool")
            require(tool in tools && seen.add(tool) && row.opt("reason") in reasons)
            for (key in listOf("raw", "menu")) {
                val count = row.opt(key) as? Int ?: error("invalid_count")
                require(count in 0..2)
            }
        }
        return value.toString()
    }
}
