package com.elon.app

internal data class WebChatProductionComposerSnapshot(
    val providerId: WebChatProviderId,
    val section: String,
    val updatedAtMs: Long,
    val options: List<WebChatConsumerOption>,
)

internal data class WebChatProductionFeatureSnapshot(
    val providerId: WebChatProviderId,
    val updatedAtMs: Long,
    val features: List<WebChatConsumerFeature>,
)

internal data class WebChatProductionInteractionSnapshot(
    val composer: List<WebChatProductionComposerSnapshot>,
    val features: List<WebChatProductionFeatureSnapshot>,
) {
    companion object {
        val EMPTY = WebChatProductionInteractionSnapshot(emptyList(), emptyList())
    }
}

/**
 * Stable native choices shown before the official page has produced its first snapshot.
 * Preset ids are presentation-only; a page action still resolves against a live semantic id.
 */
internal object WebChatProductionBuiltInCatalog {
    const val PRESET_ID_PREFIX = "preset:"

    fun composerOptions(
        providerId: WebChatProviderId,
        section: String,
    ): List<WebChatConsumerOption> = when (providerId to section.normalizedSection()) {
        WebChatProviderId.CHATGPT_WEB to MODEL_SECTION -> CHATGPT_MODELS
        WebChatProviderId.CHATGPT_WEB to TOOLS_SECTION -> CHATGPT_TOOLS
        else -> emptyList()
    }

    fun features(providerId: WebChatProviderId): List<WebChatConsumerFeature> = when (providerId) {
        WebChatProviderId.CHATGPT_WEB -> CHATGPT_FEATURES
        WebChatProviderId.GOOGLE_WEB -> emptyList()
    }

    fun isPresetId(id: String): Boolean = id.startsWith(PRESET_ID_PREFIX)

    private val CHATGPT_MODELS = listOf(
        WebChatConsumerOption(
            id = "${PRESET_ID_PREFIX}chatgpt:model:advanced",
            label = "高级",
            selected = false,
            semantic = "model",
            opensSubmenu = true,
            nativeSelector = "web-chat-model-advanced",
        ),
        WebChatConsumerOption(
            id = "${PRESET_ID_PREFIX}chatgpt:model:auto",
            label = "自动",
            selected = false,
            semantic = "model",
            opensSubmenu = false,
            nativeSelector = "web-chat-model-preset:auto",
        ),
    )

    private val CHATGPT_TOOLS = listOf(
        composerTool("image-generation", "创建图片", "image_generation"),
        composerTool("web-search", "网页搜索", "web_search"),
    )

    private val CHATGPT_FEATURES = listOf(
        WebChatConsumerFeature(
            id = stableFeatureId("图像", "images", "/images", 0),
            label = "图像",
            kind = "images",
            selected = false,
            requiresUserConfirmation = false,
            nativeSelector = "web-chat-feature:images",
        ),
    )

    private fun composerTool(
        id: String,
        label: String,
        semantic: String,
    ) = WebChatConsumerOption(
        id = "${PRESET_ID_PREFIX}chatgpt:tool:$id",
        label = label,
        selected = false,
        semantic = semantic,
        opensSubmenu = false,
        nativeSelector = "web-chat-composer-tool:$semantic",
    )

    /** Matches the official-page adapter's deterministic FNV-1a feature id. */
    private fun stableFeatureId(
        label: String,
        kind: String,
        path: String,
        occurrence: Int,
    ): String {
        var hash = 0x811c9dc5L
        "$kind|$label|$path|$occurrence".forEach { character ->
            hash = (hash xor character.code.toLong()) * 0x01000193L and 0xffff_ffffL
        }
        return "feature_${hash.toString(36)}"
    }

    private fun String.normalizedSection(): String = trim().lowercase()

    private const val MODEL_SECTION = "model"
    private const val TOOLS_SECTION = "tools"
}
