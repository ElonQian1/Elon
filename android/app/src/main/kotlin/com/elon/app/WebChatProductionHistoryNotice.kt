package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal object WebChatProductionHistoryNotice {
    fun prepend(
        snapshot: ChatGptWebSnapshot,
        provider: WebChatProviderIdentity,
        messages: List<ChatMessage>,
        timestampFor: (String) -> Long,
    ): List<ChatMessage> {
        val unavailableCount = snapshot.messageWindowStart.coerceAtLeast(0)
        if (unavailableCount == 0) return messages

        val sourceMessageId = "history-before-$unavailableCount"
        val id = "${provider.id.wireValue}:$sourceMessageId"
        val visibleCount = snapshot.messages.size
        val content = if (visibleCount > 0) {
            "当前显示最近 $visibleCount 条消息，较早的 $unavailableCount 条消息未在本机加载。"
        } else {
            "较早的 $unavailableCount 条消息暂未在本机加载。"
        }
        val notice = ChatMessage(
            role = "friend",
            content = content,
            senderLabel = provider.displayName,
            id = id,
            senderAvatarResId = provider.avatarResId,
            createdAtMs = timestampFor(id),
            webChatMessage = WebChatProductionMessage(
                providerWireValue = provider.id.wireValue,
                sourceMessageId = sourceMessageId,
                actions = emptySet(),
                contentParts = listOf(
                    WebChatProductionContentPart(
                        type = "interactive",
                        label = "在官网查看完整会话",
                    ),
                ),
            ),
        )
        return buildList(messages.size + 1) {
            add(notice)
            addAll(messages)
        }
    }
}
