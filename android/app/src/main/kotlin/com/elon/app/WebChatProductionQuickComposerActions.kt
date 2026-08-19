package com.elon.app

internal enum class WebChatProductionQuickComposerAction(
    val semantic: String,
    val label: String,
) {
    IMAGE_GENERATION("image_generation", "创建图片"),
    WEB_SEARCH("web_search", "网页搜索"),
}

internal object WebChatProductionQuickComposerActionCatalog {
    fun availableFor(provider: WebChatProviderIdentity): List<WebChatProductionQuickComposerAction> =
        if (provider.supports(WebChatProviderCapability.COMPOSER_TOOLS)) {
            WebChatProductionQuickComposerAction.entries
        } else {
            emptyList()
        }
}

internal object WebChatProductionQuickComposerActionResolver {
    fun find(
        action: WebChatProductionQuickComposerAction,
        tools: List<WebChatProductionComposerTool>,
        sourceOptions: List<WebChatConsumerOption>,
    ): WebChatProductionComposerTool? {
        val semanticById = sourceOptions.associate { it.id to it.semantic }
        return tools.firstOrNull { tool ->
            matches(action, listOf(tool.id, tool.label, semanticById[tool.id].orEmpty()))
        }
    }

    private fun matches(
        action: WebChatProductionQuickComposerAction,
        signals: List<String>,
    ): Boolean {
        val normalized = signals.joinToString(" ")
            .lowercase()
            .replace(Regex("[\\s_/-]+"), "")
        return when (action) {
            WebChatProductionQuickComposerAction.IMAGE_GENERATION ->
                "imagegeneration" in normalized ||
                    "createimage" in normalized ||
                    "创建图片" in normalized ||
                    "生成图片" in normalized ||
                    "创建图像" in normalized ||
                    "生成图像" in normalized

            WebChatProductionQuickComposerAction.WEB_SEARCH ->
                "websearch" in normalized ||
                    "searchtheweb" in normalized ||
                    "网页搜索" in normalized ||
                    "联网搜索" in normalized
        }
    }
}
