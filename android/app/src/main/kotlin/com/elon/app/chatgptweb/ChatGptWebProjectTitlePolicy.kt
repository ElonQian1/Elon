package com.elon.app.chatgptweb

import java.util.Locale

internal object ChatGptWebProjectTitlePolicy {
    fun normalize(value: String?): String? = value
        ?.trim()
        ?.replace(WHITESPACE, " ")
        ?.takeIf { it.isNotBlank() && it.length <= MAX_TITLE_LENGTH }
        ?.takeUnless { it.lowercase(Locale.ROOT) in RESERVED_TITLES }

    fun prefer(previous: String?, observed: String?): String? =
        normalize(previous) ?: normalize(observed)

    private const val MAX_TITLE_LENGTH = 160
    private val WHITESPACE = Regex("\\s+")
    private val RESERVED_TITLES = setOf(
        "chat",
        "chatgpt",
        "projects",
        "project",
        "new project",
        "null",
        "聊天",
        "项目",
        "新建项目",
        "新项目",
    )
}
