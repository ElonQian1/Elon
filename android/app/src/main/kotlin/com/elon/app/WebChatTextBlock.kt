package com.elon.app

import org.json.JSONObject

data class WebChatTextBlock(
    val id: String,
    val kind: String,
    val title: String,
    val language: String,
    val content: String,
    val complete: Boolean,
    val sourceMessageId: String? = null,
) {
    fun toJson(): JSONObject = JSONObject().put("version", 1).put("id", id).put("kind", kind)
        .put("title", title).put("language", language).put("content", content).put("complete", complete)
        .put("sourceMessageId", sourceMessageId)

    companion object {
        const val MAX_CONTENT = 120_000

        fun parse(value: JSONObject?, streaming: Boolean = false): WebChatTextBlock? {
            if (value == null || value.optInt("version") != 1) return null
            val id = (value.opt("id") as? String)?.takeIf { it.matches(Regex("[A-Za-z0-9_.:+#-]{1,160}")) }
                ?: return null
            val kind = value.optString("kind").takeIf { it == "writing" || it == "code" } ?: return null
            val body = (value.opt("content") as? String)?.takeIf { it.length <= MAX_CONTENT } ?: return null
            val language = value.optString("language").takeIf { it.matches(Regex("[A-Za-z0-9_+.#-]{1,32}")) }.orEmpty()
            return WebChatTextBlock(id, kind, value.optString("title").take(160), language, body,
                value.opt("complete") == true && !streaming,
                (value.opt("sourceMessageId") as? String)?.takeIf { kind == "writing" && it.matches(Regex("[A-Za-z0-9_-]{1,128}")) })
        }
    }
}
