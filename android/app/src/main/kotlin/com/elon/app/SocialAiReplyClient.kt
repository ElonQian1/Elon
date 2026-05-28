package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URLEncoder

internal fun requestFriendSelectedAiReply(
    http: OkHttpClient,
    serverUrl: String,
    activity: AppCompatActivity,
    friendId: String,
    messageId: String
) {
    postSelectedAiReply(
        http = http,
        activity = activity,
        url = "$serverUrl/api/me/friends/${socialAiUrlPart(friendId)}/messages/${socialAiUrlPart(messageId)}/ai-reply"
    )
}

internal fun requestGroupSelectedAiReply(
    http: OkHttpClient,
    serverUrl: String,
    activity: AppCompatActivity,
    groupId: String,
    messageId: String
) {
    postSelectedAiReply(
        http = http,
        activity = activity,
        url = "$serverUrl/api/me/groups/${socialAiUrlPart(groupId)}/messages/${socialAiUrlPart(messageId)}/ai-reply"
    )
}

private fun postSelectedAiReply(
    http: OkHttpClient,
    activity: AppCompatActivity,
    url: String
) {
    val payload = "{}".toRequestBody("application/json".toMediaType())
    val request = AuthManager.applyAuth(
        activity,
        Request.Builder()
            .url(url)
            .post(payload)
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) {
            error(socialAiErrorMessage(body, "AI回复触发失败"))
        }
    }
}

private fun socialAiErrorMessage(body: String, fallback: String): String {
    if (body.isBlank()) return fallback
    return runCatching {
        JSONObject(body).optString("error", "").ifBlank { fallback }
    }.getOrDefault(fallback)
}

private fun socialAiUrlPart(value: String): String {
    return URLEncoder.encode(value, Charsets.UTF_8.name())
}
