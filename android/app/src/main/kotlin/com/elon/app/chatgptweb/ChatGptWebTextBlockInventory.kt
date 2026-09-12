package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebTextBlockInventory {
    const val SCHEMA = "elon.text_block_inventory.v1"
    private val counts = setOf("messages", "code_blocks", "writing_blocks", "writable_blocks",
        "metadata_messages", "unparsed_messages")
    private val statuses = setOf("ready", "busy", "context_unavailable", "context_changed", "invalid_response", "read_failed")

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == counts + setOf("schema", "status", "bounded"))
        require(value.opt("schema") == SCHEMA && value.opt("status") in statuses && value.opt("bounded") == true)
        for (key in counts) {
            val count = value.opt(key)
            require((count is Int || count is Long) && (count as Number).toLong() in 0L..1280L)
            if (value.opt("status") != "ready") require((count as Number).toInt() == 0)
        }
        require(value.getInt("messages") <= 80)
        require(value.getInt("metadata_messages") <= value.getInt("messages"))
        require(value.getInt("unparsed_messages") <= value.getInt("messages"))
        require(value.getInt("writable_blocks") <= value.getInt("writing_blocks"))
        require(value.getInt("code_blocks") + value.getInt("writing_blocks") <= value.getInt("messages") * 16)
        return value.toString()
    }
}
