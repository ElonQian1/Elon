package com.elon.app

import org.json.JSONObject

internal fun jsonStringOrNull(json: JSONObject, name: String): String? {
    if (!json.has(name) || json.isNull(name)) return null
    return json.optString(name, "")
        .trim()
        .takeIf { it.isNotBlank() && it != "null" }
}

internal fun jsonStringOrNull(json: com.google.gson.JsonObject, name: String): String? {
    val element = json.get(name) ?: return null
    if (element.isJsonNull) return null
    return runCatching { element.asString }.getOrNull()
}
