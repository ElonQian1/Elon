package com.elon.app

import org.json.JSONObject

internal fun taskPayloadTraceId(payload: String?): String? {
    return taskPayloadString(payload, "trace_id")
}

internal fun taskPayloadString(payload: String?, key: String): String? {
    return payload
        ?.let { runCatching { JSONObject(it).optString(key) }.getOrNull() }
        ?.takeIf { it.isNotBlank() }
}

internal fun taskJsonStringOrNull(json: JSONObject, key: String): String? {
    if (!json.has(key) || json.isNull(key)) return null
    return json.optString(key)
        .trim()
        .takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

internal fun isTaskChatReplyType(type: String?): Boolean {
    return type in setOf("progress", "done", "error", "task_event", "message", "assistant_message")
}

internal fun taskTextPreview(value: String, maxChars: Int = 160): String {
    val singleLine = value.replace('\n', ' ').trim()
    return if (singleLine.length <= maxChars) {
        singleLine
    } else {
        singleLine.take(maxChars) + "..."
    }
}
