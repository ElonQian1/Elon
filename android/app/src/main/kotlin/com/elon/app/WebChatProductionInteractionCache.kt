package com.elon.app

/**
 * Keeps the last user-visible interaction manifest for each web provider.
 * A refresh never removes the previous snapshot, so production UI can render
 * immediately and validate the cached semantic id only when it is selected.
 */
internal class WebChatProductionInteractionCache {
    constructor(
        storage: WebChatProductionInteractionSnapshotStorage? = null,
        nowMs: () -> Long = System::currentTimeMillis,
    ) {
        this.storage = storage
        this.nowMs = nowMs
        restore(storage?.restore())
    }

    private var storage: WebChatProductionInteractionSnapshotStorage? = null
    private var nowMs: () -> Long = System::currentTimeMillis
    private val composerSections = mutableMapOf<ComposerKey, CacheEntry<WebChatConsumerOption>>()
    private val features = mutableMapOf<WebChatProviderId, CacheEntry<WebChatConsumerFeature>>()
    private val controls = mutableMapOf<ControlKey, CacheEntry<WebChatConsumerControlDescriptor>>()

    fun composerOptions(
        providerId: WebChatProviderId,
        section: String,
        observed: List<WebChatConsumerOption>,
    ): List<WebChatConsumerOption> {
        val key = ComposerKey(providerId, section.normalizedSection())
        if (record(composerSections, key, observed)) persist()
        return composerSections[key]?.values.orEmpty().ifEmpty {
            WebChatProductionBuiltInCatalog.composerOptions(providerId, key.section)
        }
    }

    fun replaceComposerOptions(
        providerId: WebChatProviderId,
        section: String,
        observed: List<WebChatConsumerOption>,
    ): List<WebChatConsumerOption> = composerOptions(providerId, section, observed)

    fun hasComposerSnapshot(providerId: WebChatProviderId, section: String): Boolean =
        composerOptions(providerId, section, emptyList()).isNotEmpty()

    fun hasFeatureSnapshot(providerId: WebChatProviderId): Boolean =
        features(providerId, emptyList()).isNotEmpty()

    fun hasControlSnapshot(providerId: WebChatProviderId, state: WebChatConsumerState): Boolean =
        controls[controlKey(providerId, state)]?.values.orEmpty().isNotEmpty()

    fun needsComposerRefresh(providerId: WebChatProviderId, section: String): Boolean =
        !composerSections[ComposerKey(providerId, section.normalizedSection())].isFresh(
            nowMs(),
            STABLE_CATALOG_FRESH_MS,
        )

    fun needsFeatureRefresh(providerId: WebChatProviderId): Boolean =
        !features[providerId].isFresh(nowMs(), STABLE_CATALOG_FRESH_MS)

    fun needsControlRefresh(providerId: WebChatProviderId, state: WebChatConsumerState): Boolean =
        !controls[controlKey(providerId, state)].isFresh(nowMs(), CONTEXT_CONTROL_FRESH_MS)

    fun features(
        providerId: WebChatProviderId,
        observed: List<WebChatConsumerFeature>,
    ): List<WebChatConsumerFeature> {
        if (record(features, providerId, observed)) persist()
        return features[providerId]?.values.orEmpty().ifEmpty {
            WebChatProductionBuiltInCatalog.features(providerId)
        }
    }

    fun controls(
        providerId: WebChatProviderId,
        state: WebChatConsumerState,
    ): List<WebChatConsumerControlDescriptor> {
        val key = controlKey(providerId, state)
        if (state.controls.isNotEmpty()) {
            controls[key] = CacheEntry(state.controls.toList(), nowMs())
        }
        return state.controls.ifEmpty { controls[key]?.values.orEmpty() }
    }

    fun capture(providerId: WebChatProviderId, state: WebChatConsumerState) {
        var shouldPersist = false
        state.composerSections.forEach { (section, options) ->
            shouldPersist = record(
                composerSections,
                ComposerKey(providerId, section.normalizedSection()),
                options,
            ) || shouldPersist
        }
        shouldPersist = record(features, providerId, state.features) || shouldPersist
        controls(providerId, state)
        if (shouldPersist) persist()
    }

    fun clear(providerId: WebChatProviderId) {
        composerSections.keys.removeAll { it.providerId == providerId }
        features.remove(providerId)
        controls.keys.removeAll { it.providerId == providerId }
        persist()
    }

    private fun restore(snapshot: WebChatProductionInteractionSnapshot?) {
        snapshot ?: return
        val now = nowMs()
        snapshot.composer.forEach { group ->
            if (group.updatedAtMs.isRetained(now) && group.options.isNotEmpty()) {
                composerSections[ComposerKey(group.providerId, group.section.normalizedSection())] =
                    CacheEntry(group.options.toList(), group.updatedAtMs)
            }
        }
        snapshot.features.forEach { group ->
            if (group.updatedAtMs.isRetained(now) && group.features.isNotEmpty()) {
                features[group.providerId] = CacheEntry(group.features.toList(), group.updatedAtMs)
            }
        }
    }

    private fun persist() {
        storage?.save(WebChatProductionInteractionSnapshot(
            composer = composerSections.map { (key, entry) ->
                WebChatProductionComposerSnapshot(
                    providerId = key.providerId,
                    section = key.section,
                    updatedAtMs = entry.updatedAtMs,
                    options = entry.values,
                )
            },
            features = features.map { (providerId, entry) ->
                WebChatProductionFeatureSnapshot(
                    providerId = providerId,
                    updatedAtMs = entry.updatedAtMs,
                    features = entry.values,
                )
            },
        ))
    }

    private fun <K, T> record(
        cache: MutableMap<K, CacheEntry<T>>,
        key: K,
        observed: List<T>,
    ): Boolean {
        if (observed.isEmpty()) return false
        val now = nowMs()
        val previous = cache[key]
        cache[key] = CacheEntry(observed.toList(), now)
        return previous == null || previous.values != observed ||
            now - previous.updatedAtMs >= PERSIST_REFRESH_INTERVAL_MS
    }

    private fun <T> CacheEntry<T>?.isFresh(now: Long, maxAgeMs: Long): Boolean =
        this != null && now - updatedAtMs in 0..maxAgeMs && values.isNotEmpty()

    private fun Long.isRetained(now: Long): Boolean = now - this in 0..MAX_RETENTION_MS

    private fun String.normalizedSection(): String = trim().lowercase()

    private data class CacheEntry<T>(
        val values: List<T>,
        val updatedAtMs: Long,
    )

    private data class ComposerKey(
        val providerId: WebChatProviderId,
        val section: String,
    )

    private data class ControlKey(
        val providerId: WebChatProviderId,
        val pageKey: String,
    )

    private fun controlKey(
        providerId: WebChatProviderId,
        state: WebChatConsumerState,
    ) = ControlKey(providerId, WebChatProductionPageIdentity.from(state).cacheKey)

    private companion object {
        const val STABLE_CATALOG_FRESH_MS = 6L * 60L * 60L * 1_000L
        const val CONTEXT_CONTROL_FRESH_MS = 2L * 60L * 1_000L
        const val PERSIST_REFRESH_INTERVAL_MS = 30L * 60L * 1_000L
        const val MAX_RETENTION_MS = 30L * 24L * 60L * 60L * 1_000L
    }
}

internal object WebChatProductionInteractionPlaceholder {
    fun item(
        providerId: WebChatProviderId,
        surface: String,
        title: String,
        state: WebChatProductionObservationState = WebChatProductionObservationState.SYNCING,
    ) = WebChatActionSheetItem(
        id = "status:$surface:${WebChatProductionCapabilityEvidencePolicy.selectorState(state)}",
        title = title,
        subtitle = WebChatProductionCapabilityEvidencePolicy.subtitle(state),
        enabled = false,
        contentDescription = "web-chat-$surface-" +
            "${WebChatProductionCapabilityEvidencePolicy.selectorState(state)}:${providerId.wireValue}",
    )
}
