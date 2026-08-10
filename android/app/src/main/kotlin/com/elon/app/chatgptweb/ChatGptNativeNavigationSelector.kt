package com.elon.app.chatgptweb

internal object ChatGptNativeNavigationSelector {
    const val SCHEMA = "elon.chatgpt_web.native_navigation.v1"

    fun conversation(value: ChatGptWebConversation): String = selector(
        prefix = "chatgpt-conversation",
        id = value.id,
        label = value.title,
    )

    fun feature(value: ChatGptWebFeature): String = selector(
        prefix = "chatgpt-feature",
        id = value.id,
        label = value.label,
    )

    fun composerOption(section: String, value: ChatGptWebComposerOption): String = selector(
        prefix = "chatgpt-composer-option:${stableToken(section)}",
        id = value.id,
        label = value.label,
    )

    fun composerDialog(section: String): String =
        "chatgpt-composer-options:${stableToken(section)}"

    private fun selector(prefix: String, id: String, label: String): String =
        "$prefix:${stableToken(id)}:${stableLabel(label)}".take(MAX_SELECTOR_LENGTH)

    private fun stableToken(value: String): String = value
        .trim()
        .replace(Regex("[^A-Za-z0-9_.-]"), "_")
        .take(MAX_TOKEN_LENGTH)
        .ifBlank { "unknown" }

    private fun stableLabel(value: String): String = value
        .trim()
        .replace(Regex("\\s+"), " ")
        .take(MAX_LABEL_LENGTH)
        .ifBlank { "未命名" }

    private const val MAX_TOKEN_LENGTH = 96
    private const val MAX_LABEL_LENGTH = 120
    private const val MAX_SELECTOR_LENGTH = 240
}
