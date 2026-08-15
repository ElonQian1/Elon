package com.elon.app.chatgptweb

import com.elon.app.ChatMessage
import com.elon.app.ChatAttachment
import com.elon.app.WebChatProviderIdentity

internal object ChatGptFriendMessageMapper {
    fun map(
        snapshot: ChatGptWebSnapshot,
        provider: WebChatProviderIdentity,
        pendingPrompt: String?,
        pendingAttachments: List<ChatAttachment> = emptyList(),
        pendingSendStatus: String = "发送中…",
        attachmentsForMessage: (String) -> List<ChatAttachment> = { emptyList() },
        messageActionContextIds: Set<String> = emptySet(),
        timestampFor: (String) -> Long,
    ): List<ChatMessage> {
        val latestAssistantIndex = snapshot.messages.indexOfLast { it.role == "assistant" }
        val result = snapshot.messages.mapIndexed { index, message ->
            val id = "${provider.id.wireValue}:${message.id}"
            val messageAttachments = attachmentsForMessage(message.id).ifEmpty {
                message.parts.mapNotNull(::attachmentFromPart)
            }
            val actions = buildSet {
                if (
                    message.content.isNotBlank() &&
                    provider.supports(com.elon.app.WebChatProviderCapability.MESSAGE_COPY)
                ) {
                    add(com.elon.app.WebChatMessageAction.COPY)
                }
                if (
                    index == latestAssistantIndex &&
                    message.role == "assistant" &&
                    message.state == "completed" &&
                    !snapshot.streaming &&
                    provider.supports(com.elon.app.WebChatProviderCapability.MESSAGE_REGENERATE) &&
                    snapshot.capabilities.supports(ChatGptWebCapabilityId.MESSAGE_REGENERATE)
                ) {
                    add(com.elon.app.WebChatMessageAction.REGENERATE)
                }
                if (
                    provider.supports(com.elon.app.WebChatProviderCapability.MESSAGE_CONTEXT_ACTIONS) &&
                    ChatGptNativeControlPresentation.stableContextId(message.id) in messageActionContextIds
                ) {
                    add(com.elon.app.WebChatMessageAction.MORE)
                }
            }
            ChatMessage(
                role = if (message.role == "user") "user" else "friend",
                content = message.content,
                senderLabel = if (message.role == "assistant") provider.displayName else null,
                id = id,
                senderAvatarResId = if (message.role == "assistant") provider.avatarResId else null,
                createdAtMs = timestampFor(id),
                modelUsed = snapshot.currentModel.takeIf { message.role == "assistant" && it.isNotBlank() },
                attachments = messageAttachments.takeIf(List<ChatAttachment>::isNotEmpty),
                webChatMessage = com.elon.app.WebChatProductionMessage(
                    providerWireValue = provider.id.wireValue,
                    sourceMessageId = message.id,
                    actions = actions,
                ).takeIf { actions.isNotEmpty() },
            )
        }.toMutableList()

        val cleanPending = pendingPrompt?.trim().orEmpty()
        val promptObserved = snapshot.messages.lastOrNull { it.role == "user" }
            ?.content
            ?.trim() == cleanPending
        if ((cleanPending.isNotEmpty() || pendingAttachments.isNotEmpty()) && !promptObserved) {
            val id = "${provider.id.wireValue}:pending_user"
            result += ChatMessage(
                role = "user",
                content = cleanPending,
                sendStatus = pendingSendStatus,
                id = id,
                createdAtMs = timestampFor(id),
                attachments = pendingAttachments.takeIf(List<ChatAttachment>::isNotEmpty),
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

    private fun attachmentFromPart(part: ChatGptWebMessagePart): ChatAttachment? = when (part.type) {
        "image" -> ChatAttachment(
            kind = "image",
            displayName = part.label,
            mimeType = part.metadata?.mediaType,
        )
        "file" -> ChatAttachment(
            kind = "file",
            displayName = part.label,
            mimeType = part.metadata?.mediaType,
        )
        else -> null
    }
}
