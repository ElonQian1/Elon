package com.elon.app.chatgptweb

import com.elon.app.ChatMessage
import com.elon.app.WebChatProviderIdentity

internal object ChatGptFriendMessageMapper {
    fun map(
        snapshot: ChatGptWebSnapshot,
        provider: WebChatProviderIdentity,
        pendingPrompt: String?,
        timestampFor: (String) -> Long,
    ): List<ChatMessage> {
        val result = snapshot.messages.map { message ->
            val id = "${provider.id.wireValue}:${message.id}"
            ChatMessage(
                role = if (message.role == "user") "user" else "friend",
                content = message.content,
                senderLabel = if (message.role == "assistant") provider.displayName else null,
                id = id,
                senderAvatarResId = if (message.role == "assistant") provider.avatarResId else null,
                createdAtMs = timestampFor(id),
                modelUsed = snapshot.currentModel.takeIf { message.role == "assistant" && it.isNotBlank() },
            )
        }.toMutableList()

        val cleanPending = pendingPrompt?.trim().orEmpty()
        val promptObserved = snapshot.messages.lastOrNull { it.role == "user" }
            ?.content
            ?.trim() == cleanPending
        if (cleanPending.isNotEmpty() && !promptObserved) {
            val id = "${provider.id.wireValue}:pending_user"
            result += ChatMessage(
                role = "user",
                content = cleanPending,
                sendStatus = "发送中…",
                id = id,
                createdAtMs = timestampFor(id),
            )
        }
        if (snapshot.streaming && result.lastOrNull()?.role != "friend") {
            val id = "${provider.id.wireValue}:streaming"
            result += ChatMessage(
                role = "friend",
                content = "正在回复…",
                senderLabel = provider.displayName,
                id = id,
                senderAvatarResId = provider.avatarResId,
                createdAtMs = timestampFor(id),
                modelUsed = snapshot.currentModel.takeIf(String::isNotBlank),
            )
        }
        return result
    }
}
