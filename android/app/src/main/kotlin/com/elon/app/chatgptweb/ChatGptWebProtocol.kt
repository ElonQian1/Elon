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
    val draft: String,
    val messages: List<ChatGptWebMessage>,
    val authenticated: Boolean,
    val composerReady: Boolean,
    val streaming: Boolean,
    val capabilities: ChatGptWebCapabilities,
)

internal sealed interface ChatGptWebEvent {
    data class Snapshot(val value: ChatGptWebSnapshot) : ChatGptWebEvent

    data class ConversationList(
        val conversations: List<ChatGptWebConversation>,
    ) : ChatGptWebEvent

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
                "conversation_snapshot" -> ChatGptWebEvent.ConversationList(
                    parseConversations(event),
                )
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
            draft = event.optString("draft").take(MAX_DRAFT_LENGTH),
            messages = messages,
            authenticated = event.optBoolean("authenticated"),
            composerReady = event.optBoolean("composerReady"),
            streaming = event.optBoolean("streaming"),
            capabilities = ChatGptWebCapabilities(parseStringSet(event, "capabilities")),
        )
    }

    private fun parseConversations(event: JSONObject): List<ChatGptWebConversation> {
        val items = event.optJSONArray("conversations") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(items.length(), MAX_CONVERSATIONS)) {
                val item = items.optJSONObject(index) ?: continue
                val path = item.optString("path").take(MAX_PATH_LENGTH)
                if (!CONVERSATION_PATH.matches(path)) continue
                val id = item.optString("id").ifBlank { path.removePrefix("/c/") }
                val title = item.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (title.isBlank()) continue
                add(
                    ChatGptWebConversation(
                        id = id.take(MAX_ID_LENGTH),
                        title = title,
                        path = path,
                        active = item.optBoolean("active"),
                    ),
                )
            }
        }
    }

    private fun parseStringSet(payload: JSONObject, key: String): Set<String> {
        val values = payload.optJSONArray(key) ?: return emptySet()
        return buildSet {
            for (index in 0 until minOf(values.length(), MAX_CAPABILITIES)) {
                val value = values.optString(index).trim().take(MAX_CAPABILITY_LENGTH)
                if (CAPABILITY_ID.matches(value)) add(value)
            }
        }
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
    private const val MAX_DRAFT_LENGTH = 20_000
    private const val MAX_CONVERSATIONS = 100
    private const val MAX_CAPABILITIES = 40
    private const val MAX_CAPABILITY_LENGTH = 48
    private const val MAX_TITLE_LENGTH = 160
    private const val MAX_PATH_LENGTH = 256
    private const val MAX_ID_LENGTH = 160
    private val CAPABILITY_ID = Regex("[a-z][a-z0-9_]{0,47}")
    private val CONVERSATION_PATH = Regex("/c/[A-Za-z0-9_-]{1,160}")
}
