package com.elon.app.chatgptweb

internal object ChatGptWebControlSemanticNormalizer {
    fun normalize(
        raw: String,
        label: String,
        region: String,
        role: String,
    ): String {
        val semantic = raw.takeIf { it in ChatGptWebUiSemantics.KNOWN }
            ?: ChatGptWebUiSemantics.GENERIC_ACTION
        if (
            semantic == ChatGptWebUiSemantics.GENERIC_ACTION &&
            region == ChatGptWebUiRegion.COMPOSER &&
            WEB_SEARCH_LABEL.matches(label.trim())
        ) {
            return ChatGptWebUiSemantics.WEB_SEARCH
        }
        if (
            semantic == ChatGptWebUiSemantics.GENERIC_ACTION &&
            role == "link"
        ) {
            return ChatGptWebUiSemantics.OPEN_LINK
        }
        return semantic
    }

    private val WEB_SEARCH_LABEL = Regex(
        "^(?:search|搜索|search the web|web search|browse|网页搜索|联网搜索)$",
        RegexOption.IGNORE_CASE,
    )
}
