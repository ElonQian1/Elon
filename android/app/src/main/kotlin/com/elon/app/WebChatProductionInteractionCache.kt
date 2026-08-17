package com.elon.app

/**
 * Keeps the last user-visible interaction manifest for each web provider.
 * A refresh never removes the previous snapshot, so production UI can render
 * immediately and validate the cached semantic id only when it is selected.
 */
internal class WebChatProductionInteractionCache {
    private val composerSections = mutableMapOf<ComposerKey, List<WebChatConsumerOption>>()
    private val features = mutableMapOf<WebChatProviderId, List<WebChatConsumerFeature>>()
    private val controls = mutableMapOf<WebChatProviderId, List<WebChatConsumerControlDescriptor>>()

    fun composerOptions(
        providerId: WebChatProviderId,
        section: String,
        observed: List<WebChatConsumerOption>,
    ): List<WebChatConsumerOption> = retainLatest(
        key = ComposerKey(providerId, section.trim().lowercase()),
        observed = observed,
        cache = composerSections,
    )

    fun features(
        providerId: WebChatProviderId,
        observed: List<WebChatConsumerFeature>,
    ): List<WebChatConsumerFeature> = retainLatest(providerId, observed, features)

    fun controls(
        providerId: WebChatProviderId,
        observed: List<WebChatConsumerControlDescriptor>,
    ): List<WebChatConsumerControlDescriptor> = retainLatest(providerId, observed, controls)

    fun clear(providerId: WebChatProviderId) {
        composerSections.keys.removeAll { it.providerId == providerId }
        features.remove(providerId)
        controls.remove(providerId)
    }

    private fun <K, T> retainLatest(
        key: K,
        observed: List<T>,
        cache: MutableMap<K, List<T>>,
    ): List<T> {
        if (observed.isNotEmpty()) cache[key] = observed.toList()
        return observed.ifEmpty { cache[key].orEmpty() }
    }

    private data class ComposerKey(
        val providerId: WebChatProviderId,
        val section: String,
    )
}

internal object WebChatProductionInteractionPlaceholder {
    fun item(
        providerId: WebChatProviderId,
        surface: String,
        title: String,
    ) = WebChatActionSheetItem(
        id = "sync:$surface",
        title = title,
        subtitle = "后台同步中",
        enabled = false,
        contentDescription = "web-chat-$surface-syncing:${providerId.wireValue}",
    )
}
