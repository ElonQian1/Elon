package com.elon.app

import android.content.Context
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URLEncoder

internal class GroupRevisionFailure(val status: Int, message: String) : Exception(message)

internal class GroupMessageRevisionApi(private val context: Context, private val http: OkHttpClient, private val serverUrl: String) {
    fun save(groupId: String, messageId: String, content: String, revision: Long): JSONObject {
        val payload = JSONObject().put("content", content.trim()).put("expected_revision", revision)
        return execute(Request.Builder().url(path(groupId, messageId)).patch(payload.toString().toRequestBody("application/json".toMediaType()))).getJSONObject("message")
    }

    fun history(groupId: String, messageId: String, before: Long? = null, limit: Int = 20): JSONObject {
        val query = "/revisions?limit=$limit" + (before?.let { "&before_revision=$it" } ?: "")
        return execute(Request.Builder().url(path(groupId, messageId) + query).get())
    }

    private fun path(groupId: String, messageId: String): String = "$serverUrl/api/me/groups/${encode(groupId)}/messages/${encode(messageId)}"
    private fun encode(value: String) = URLEncoder.encode(value, "UTF-8")

    private fun execute(builder: Request.Builder): JSONObject {
        http.newCall(AuthManager.applyAuth(context, builder).build()).execute().use { response ->
            val data = runCatching { JSONObject(response.body?.string().orEmpty()) }.getOrDefault(JSONObject())
            if (!response.isSuccessful) throw GroupRevisionFailure(response.code, data.optString("error", "请求失败，请重试"))
            return data
        }
    }
}
