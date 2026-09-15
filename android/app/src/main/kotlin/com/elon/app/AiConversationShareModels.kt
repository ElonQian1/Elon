package com.elon.app

data class AiConversationShareCard(
    val id: String,
    val groupId: String,
    val title: String,
    val summary: String,
    val provider: String,
    val senderName: String,
    val messageCount: Int,
    val coverAssetId: String? = null,
)

internal data class AiConversationShareSnapshot(
    val card: AiConversationShareCard,
    val messages: List<ChatMessage>,
    val revoked: Boolean = false,
    val ownerId: String = "",
    val gaps: Set<Int> = emptySet(),
)

internal data class AiConversationShareDraft(
    val provider: String,
    val title: String,
    val summary: String,
    val messages: List<ChatMessage>,
    val gaps: Set<Int>,
)
