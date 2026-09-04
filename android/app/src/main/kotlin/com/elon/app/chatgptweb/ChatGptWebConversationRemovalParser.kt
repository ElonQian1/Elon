package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebConversationRemovalParser {
    private const val MAX_IDS = 100
    private const val MAX_ID_LENGTH = 160
    private val ID = Regex("[A-Za-z0-9_-]{1,160}")

    fun parse(payload: JSONObject, key: String): Set<String> {
        val values = payload.optJSONArray(key) ?: return emptySet()
        return buildSet {
            for (index in 0 until minOf(values.length(), MAX_IDS)) {
                val value = values.optString(index).trim().take(MAX_ID_LENGTH)
                if (ID.matches(value)) add(value)
            }
        }
    }
}
