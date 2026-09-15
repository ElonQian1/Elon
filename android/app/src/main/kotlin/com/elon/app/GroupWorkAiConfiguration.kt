package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal data class GroupWorkAiConfiguration(val agent: String? = null,
    val label: String = "服务器默认", val allowFallback: Boolean = true) {
    fun request() = JSONObject().put("agent", agent ?: JSONObject.NULL).put("allow_fallback", allowFallback)
    fun stored() = request().put("label", label)

    companion object {
        fun decode(json: JSONObject?) = GroupWorkAiConfiguration(
            json?.optString("agent")?.takeIf { it != "null" && it.isNotBlank() && it.length <= 160 },
            json?.optString("label")?.takeIf { it.isNotBlank() }?.take(120) ?: "服务器默认",
            json?.optBoolean("allow_fallback", true) ?: true,
        )
    }
}

internal data class GroupWorkAiModel(val id: String, val label: String, val model: String) {
    fun stored() = JSONObject().put("id", id).put("label", label).put("model", model)
    companion object {
        fun parse(rows: JSONArray): List<GroupWorkAiModel> = (0 until rows.length().coerceAtMost(80)).mapNotNull {
            val row = rows.optJSONObject(it) ?: return@mapNotNull null
            val id = row.optString("id").takeIf { it.isNotBlank() && it.length <= 160 } ?: return@mapNotNull null
            GroupWorkAiModel(id, row.optString("label").ifBlank { id }.take(120), row.optString("model").take(160))
        }.distinctBy { it.id }
    }
}
