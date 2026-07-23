package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder
import kotlin.concurrent.thread

internal class ChatSocialSidebarMessageLoader(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String
) {
    fun loadLatestIncoming(
        item: SocialSidebarTimelineItem,
        onDone: (Result<ChatMessage>) -> Unit
    ) {
        thread(name = "social-sidebar-message") {
            val result = runCatching {
                val path = when (item.key.type) {
                    SocialSidebarConversationType.FRIEND ->
                        "/api/me/friends/${urlPart(item.key.id)}/messages?limit=40&preserve_unread=true"
                    SocialSidebarConversationType.GROUP ->
                        "/api/me/groups/${urlPart(item.key.id)}/messages?limit=40&preserve_unread=true"
                }
                val request = AuthManager.applyAuth(
                    activity,
                    Request.Builder().url("$serverUrl$path").get()
                ).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) {
                        error(readErrorMessage(body, "读取最新消息失败"))
                    }
                    val messages = JSONObject(body).optJSONArray("messages") ?: JSONArray()
                    List(messages.length()) { index -> messages.optJSONObject(index) }
                        .asReversed()
                        .firstNotNullOfOrNull { json ->
                            json?.takeUnless { it.optBoolean("outgoing", false) }?.toChatMessage()
                        }
                        ?: error("没有可拖拽的收到消息")
                }
            }
            activity.runOnUiThread { onDone(result) }
        }
    }

    private fun JSONObject.toChatMessage(): ChatMessage? {
        val content = optString("content")
        val attachments = chatAttachmentsFromJsonArray(optJSONArray("attachments"))
        if (content.isBlank() && attachments.isEmpty()) return null
        return ChatMessage(
            role = "friend",
            content = content,
            attachments = attachments.takeIf { it.isNotEmpty() },
            senderLabel = optString("sender_name").trim().takeIf { it.isNotEmpty() },
            id = optString("id").trim().takeIf { it.isNotEmpty() },
            createdAtMs = parseChatMessageCreatedAt(optString("created_at")) ?: System.currentTimeMillis(),
            recalledAt = cleanString("recalled_at"),
            recalledBy = cleanString("recalled_by")
        ).takeUnless { it.recalledAt != null }
    }

    private fun JSONObject.cleanString(name: String): String? =
        optString(name).trim().takeIf { it.isNotEmpty() && it != "null" }

    private fun readErrorMessage(body: String, fallback: String): String =
        runCatching { JSONObject(body).optString("error").ifBlank { fallback } }.getOrDefault(fallback)

    private fun urlPart(value: String): String =
        URLEncoder.encode(value, Charsets.UTF_8.name())
}
