package com.elon.app

private const val AUTO_CONVERSATION_TITLE_MAX_CHARS = 9

private val AUTO_CONVERSATION_PLACEHOLDER_TITLES = setOf(
    "一龙开发助手",
    "一龙项目",
    "项目开发会话",
    "等待你的第一个开发需求"
)

internal fun shouldAutoGenerateConversationTitle(conversation: AppConversation): Boolean {
    val title = conversation.title.trim()
    return title.startsWith("新会话") || title in AUTO_CONVERSATION_PLACEHOLDER_TITLES
}

internal fun autoConversationTitleFromMessage(text: String): String {
    val normalized = text.replace(Regex("\\s+"), " ").trim()
    val cleaned = normalized
        .removeConversationTitleLeadIn()
        .trimStart('：', ':', '，', ',', '。', '.', '、', ' ')
    return summarize(cleaned.ifBlank { normalized }, AUTO_CONVERSATION_TITLE_MAX_CHARS)
}

internal fun updateConversationTitleFromFirstUserMessage(conversation: AppConversation): Boolean {
    if (!shouldAutoGenerateConversationTitle(conversation)) return false
    val firstUserMessage = conversation.messages
        .firstOrNull { it.role == "user" && it.content.isNotBlank() }
        ?: return false
    val title = autoConversationTitleFromMessage(firstUserMessage.content)
    if (title.isBlank()) return false
    conversation.title = title
    return true
}

private fun String.removeConversationTitleLeadIn(): String {
    val exactPrefixes = listOf(
        "请帮我",
        "请你帮我",
        "麻烦帮我",
        "麻烦你帮我",
        "帮我",
        "我想让你",
        "我想要",
        "我想",
        "我要",
        "现在"
    )
    exactPrefixes.firstOrNull { startsWith(it) && length > it.length + 1 }?.let {
        return removePrefix(it)
    }
    val politePrefixes = listOf("请把", "请将", "请给", "请用", "请在")
    politePrefixes.firstOrNull { startsWith(it) && length > it.length + 1 }?.let {
        return removePrefix("请")
    }
    return this
}
