package com.elon.app

internal data class WebChatModelControlPresentation(
    val advanced: WebChatConsumerOption?,
    val levels: List<WebChatConsumerOption>,
    val selectedLevelIndex: Int,
    val listOptions: List<WebChatConsumerOption>,
) {
    val usesLevelSlider: Boolean get() = levels.size in 2..MAX_LEVELS

    private companion object {
        const val MAX_LEVELS = 6
    }
}

internal object WebChatModelControlPolicy {
    private val levelToken = Regex(
        "极高|重度|中度|标准|轻度|高|中|低|自动|快速|思考|extra\\s*high|high|medium|low|auto|fast|thinking",
        RegexOption.IGNORE_CASE,
    )

    fun resolve(
        options: List<WebChatConsumerOption>,
        currentModel: String,
    ): WebChatModelControlPresentation {
        val selectable = options.filter { it.id.isNotBlank() && it.label.isNotBlank() }
        val advanced = selectable.firstOrNull { it.opensSubmenu }
        val direct = selectable.filterNot(WebChatConsumerOption::opensSubmenu)
        val levels = direct.takeIf(::looksLikeLevelScale).orEmpty()
        val selectedIndex = levels.indexOfFirst(WebChatConsumerOption::selected)
            .takeIf { it >= 0 }
            ?: levels.indexOfFirst { compactLabel(it.label) == compactLabel(currentModel) }
                .takeIf { it >= 0 }
            ?: 0
        return WebChatModelControlPresentation(
            advanced = advanced,
            levels = levels,
            selectedLevelIndex = selectedIndex,
            listOptions = if (levels.isEmpty()) direct else emptyList(),
        )
    }

    fun compactLabel(raw: String): String {
        val cleaned = raw.trim().replace(Regex("\\s+"), " ")
        val token = levelToken.findAll(cleaned).lastOrNull()?.value?.lowercase()
        return when (token?.replace(" ", "")) {
            "轻度", "low" -> "低"
            "标准", "中度", "medium" -> "中"
            "重度", "high" -> "高"
            "极高", "extrahigh" -> "极高"
            "auto", "自动" -> "自动"
            "fast", "快速" -> "快速"
            "thinking", "思考" -> "思考"
            "低", "中", "高" -> token.orEmpty()
            else -> cleaned.ifBlank { "自动" }.take(MAX_COMPACT_LABEL_LENGTH)
        }
    }

    private fun looksLikeLevelScale(options: List<WebChatConsumerOption>): Boolean {
        if (options.size !in 2..6) return false
        return options.all { option ->
            val label = option.label.trim()
            label.length <= MAX_LEVEL_LABEL_LENGTH || levelToken.containsMatchIn(label)
        }
    }

    private const val MAX_COMPACT_LABEL_LENGTH = 8
    private const val MAX_LEVEL_LABEL_LENGTH = 6
}
