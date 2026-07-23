package com.elon.app

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

internal data class SocialSidebarFavorite(
    val id: String,
    val savedAt: Long,
    val message: ChatMessage
)

internal class ChatSocialFavorites(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun add(message: ChatMessage): Boolean {
        if (message.content.isBlank() && message.attachments.isNullOrEmpty()) return false
        val current = list().toMutableList()
        val stableId = message.id?.trim()?.takeIf { it.isNotEmpty() }
            ?: "${message.createdAtMs}:${message.content.hashCode()}"
        current.removeAll { it.id == stableId }
        current.add(
            0,
            SocialSidebarFavorite(
                id = stableId,
                savedAt = System.currentTimeMillis(),
                message = message.copyForSocialSidebar()
            )
        )
        val encoded = JSONArray().apply {
            current.take(MAX_ITEMS).forEach { put(it.toJson()) }
        }
        prefs.edit().putString(KEY_ITEMS, encoded.toString()).apply()
        return true
    }

    fun list(): List<SocialSidebarFavorite> {
        val raw = prefs.getString(KEY_ITEMS, null)?.takeIf { it.isNotBlank() } ?: return emptyList()
        return runCatching {
            val array = JSONArray(raw)
            List(array.length()) { index -> array.optJSONObject(index) }
                .mapNotNull { it?.toFavorite() }
                .sortedByDescending { it.savedAt }
        }.getOrDefault(emptyList())
    }

    private fun SocialSidebarFavorite.toJson(): JSONObject {
        val favorite = this
        return JSONObject().apply {
            put("id", favorite.id)
            put("saved_at", favorite.savedAt)
            put("message_id", favorite.message.id)
            put("content", favorite.message.content)
            put("created_at_ms", favorite.message.createdAtMs)
            put("attachments", JSONArray().apply {
                favorite.message.attachments.orEmpty().forEach { put(it.toJson()) }
            })
        }
    }

    private fun JSONObject.toFavorite(): SocialSidebarFavorite? {
        val favoriteId = optString("id").trim().takeIf { it.isNotEmpty() } ?: return null
        val content = optString("content")
        val attachments = optJSONArray("attachments").toChatAttachments()
        if (content.isBlank() && attachments.isEmpty()) return null
        return SocialSidebarFavorite(
            id = favoriteId,
            savedAt = optLong("saved_at", 0L).takeIf { it > 0L } ?: return null,
            message = ChatMessage(
                role = "friend",
                content = content,
                attachments = attachments.takeIf { it.isNotEmpty() },
                id = optString("message_id").trim().takeIf { it.isNotEmpty() },
                createdAtMs = optLong("created_at_ms", 0L).takeIf { it > 0L }
                    ?: System.currentTimeMillis()
            )
        )
    }

    private fun ChatAttachment.toJson(): JSONObject {
        val attachment = this
        return JSONObject().apply {
            put("kind", attachment.kind)
            put("display_name", attachment.displayName)
            put("file_name", attachment.fileName)
            put("mime_type", attachment.mimeType)
            put("url", attachment.url)
            put("local_path", attachment.localPath)
            put("size_bytes", attachment.sizeBytes)
            put("image_width", attachment.imageWidth)
            put("image_height", attachment.imageHeight)
            put("duration_seconds", attachment.durationSeconds)
            put("transcription", attachment.transcription)
        }
    }

    private fun JSONArray?.toChatAttachments(): List<ChatAttachment> {
        this ?: return emptyList()
        return List(length()) { index -> optJSONObject(index) }.mapNotNull { item ->
            item ?: return@mapNotNull null
            ChatAttachment(
                kind = item.cleanString("kind"),
                displayName = item.cleanString("display_name"),
                fileName = item.cleanString("file_name"),
                mimeType = item.cleanString("mime_type"),
                url = item.cleanString("url"),
                localPath = item.cleanString("local_path"),
                sizeBytes = item.optLong("size_bytes", 0L).takeIf { it > 0L },
                imageWidth = item.optInt("image_width", 0).takeIf { it > 0 },
                imageHeight = item.optInt("image_height", 0).takeIf { it > 0 },
                durationSeconds = item.optInt("duration_seconds", 0).takeIf { it > 0 },
                transcription = item.cleanString("transcription")
            )
        }
    }

    private fun JSONObject.cleanString(name: String): String? =
        optString(name).trim().takeIf { it.isNotEmpty() && it != "null" }

    private fun ChatMessage.copyForSocialSidebar(): ChatMessage =
        ChatMessage(
            role = role,
            content = content,
            attachments = attachments?.map { it.copy() },
            id = id,
            senderLabel = senderLabel,
            senderAvatarDataUrl = senderAvatarDataUrl,
            createdAtMs = createdAtMs
        )

    private companion object {
        const val PREFS_NAME = "chat_social_sidebar_favorites"
        const val KEY_ITEMS = "items"
        const val MAX_ITEMS = 120
    }
}
