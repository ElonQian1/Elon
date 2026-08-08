package com.elon.app.chatgptweb

import org.json.JSONObject

internal data class ChatGptWebMessage(
    val id: String,
    val role: String,
    val content: String,
)

internal data class ChatGptWebSnapshot(
    val title: String,
    val url: String,
    val messages: List<ChatGptWebMessage>,
    val authenticated: Boolean,
    val composerReady: Boolean,
    val streaming: Boolean,
)

internal sealed interface ChatGptWebEvent {
    data class Snapshot(val value: ChatGptWebSnapshot) : ChatGptWebEvent

    data class CommandResult(
        val action: String,
        val ok: Boolean,
        val detail: String,
    ) : ChatGptWebEvent
}

internal object ChatGptWebProtocol {
    fun parse(rawPayload: String): ChatGptWebEvent? {
        val payload = runCatching { JSONObject(rawPayload) }.getOrNull() ?: return null
        if (payload.optString("schema") == SCHEMA) {
            val event = payload.optJSONObject("event") ?: return null
            return when (event.optString("type")) {
                "message_snapshot" -> ChatGptWebEvent.Snapshot(parseSnapshot(event))
                else -> null
            }
        }
        return when (payload.optString("type")) {
            "command_result" -> ChatGptWebEvent.CommandResult(
                action = payload.optString("action").take(40),
                ok = payload.optBoolean("ok"),
                detail = payload.optString("detail").take(160),
            )
            else -> null
        }
    }

    private fun parseSnapshot(event: JSONObject): ChatGptWebSnapshot {
        val rawMessages = event.optJSONArray("messages")
        val messages = buildList {
            if (rawMessages == null) return@buildList
            for (index in 0 until minOf(rawMessages.length(), MAX_MESSAGES)) {
                val item = rawMessages.optJSONObject(index) ?: continue
                val role = item.optString("role").lowercase()
                if (role !in SUPPORTED_ROLES) continue
                val content = textContent(item).trim().take(MAX_MESSAGE_LENGTH)
                if (content.isEmpty()) continue
                add(
                    ChatGptWebMessage(
                        id = item.optString("id").ifBlank { "$role-$index" }.take(160),
                        role = role,
                        content = content,
                    ),
                )
            }
        }
        return ChatGptWebSnapshot(
            title = event.optString("title").trim().take(120),
            url = event.optString("url").take(2_048),
            messages = messages,
            authenticated = event.optBoolean("authenticated"),
            composerReady = event.optBoolean("composerReady"),
            streaming = event.optBoolean("streaming"),
        )
    }

    private fun textContent(message: JSONObject): String {
        val content = message.optJSONArray("content") ?: return ""
        return buildList {
            for (index in 0 until minOf(content.length(), MAX_CONTENT_PARTS)) {
                val part = content.optJSONObject(index) ?: continue
                if (part.optString("type") == "text") add(part.optString("text"))
            }
        }.joinToString("\n")
    }

    const val SCHEMA = "yilong.ai.ui.v1"
    private val SUPPORTED_ROLES = setOf("user", "assistant")
    private const val MAX_MESSAGES = 80
    private const val MAX_MESSAGE_LENGTH = 40_000
    private const val MAX_CONTENT_PARTS = 20
}
