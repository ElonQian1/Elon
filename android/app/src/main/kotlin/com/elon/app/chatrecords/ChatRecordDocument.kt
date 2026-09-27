package com.elon.app.chatrecords

import org.json.JSONArray
import org.json.JSONObject

internal const val RECORD_SCHEMA = "chat_record_bundle_v1"
internal const val RECORD_PREFIX = "【一龙聊天记录】\n"
internal data class RecordRow(
    val id: String, val parentId: String?, val sender: String, val time: String,
    val kind: String, val text: String, val filename: String = "", val assetId: String? = null
) {
    fun json() = JSONObject().put("id", id).put("parent_id", parentId ?: JSONObject.NULL)
        .put("sender", sender).put("time", time).put("kind", kind).put("text", text)
        .put("filename", filename).put("asset_id", assetId ?: JSONObject.NULL)
}
internal data class ChatRecordDocument(
    val title: String, val rawText: String, val messages: List<RecordRow>, val warnings: List<String> = emptyList()
) {
    fun json() = JSONObject().put("schema", RECORD_SCHEMA).put("source", "wechat").put("title", title)
        .put("raw_text", rawText).put("messages", JSONArray().apply { messages.forEach { put(it.json()) } })
        .put("warnings", JSONArray(warnings))
    fun children(parent: String?) = messages.filter { it.parentId == parent }
    companion object {
        fun read(json: JSONObject): ChatRecordDocument {
            require(json.getString("schema") == RECORD_SCHEMA)
            val rows = json.getJSONArray("messages")
            require(rows.length() in 1..2000)
            return ChatRecordDocument(json.getString("title"), json.optString("raw_text"), (0 until rows.length()).map {
                val r = rows.getJSONObject(it)
                RecordRow(r.getString("id"), r.optString("parent_id").takeUnless { s -> s.isBlank() || s == "null" },
                    r.getString("sender"), r.getString("time"), r.getString("kind"), r.getString("text"), r.optString("filename"),
                    r.optString("asset_id").takeUnless { s -> s.isBlank() || s == "null" })
            }, json.optJSONArray("warnings")?.let { a -> (0 until a.length()).map { a.getString(it) } }.orEmpty())
        }
        fun card(content: String): JSONObject? = runCatching {
            require(content.startsWith(RECORD_PREFIX))
            JSONObject(content.removePrefix(RECORD_PREFIX)).also {
                require(it.getString("schema") == RECORD_SCHEMA)
                require(it.getString("record_id").matches(Regex("[A-Za-z0-9_-]{1,128}")))
                require(it.getString("group_id").matches(Regex("[A-Za-z0-9_-]{1,128}")))
            }
        }.getOrNull()
    }
}
