package com.elon.app

/**
 * Best-effort, bounded prefetch for controls that should already be available when opened.
 * It never blocks the production UI and never keeps polling after a provider switch.
 */
internal class WebChatProductionCapabilityPrewarmer(
    private val consumerPort: () -> WebChatConsumerPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val interactionCache: WebChatProductionInteractionCache,
    private val scheduleAction: (delayMs: Long, action: () -> Unit) -> Unit,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private var epoch = 0
    private var runningKey: RunKey? = null
    private val nextEligibleAtMs = mutableMapOf<RunKey, Long>()

    fun schedule(provider: WebChatProviderIdentity) {
        if (requirements(provider).isEmpty()) return
        val runKey = RunKey(
            provider.id,
            consumerPort()?.state()?.let(WebChatProductionPageIdentity::from)?.cacheKey ?: "unknown:/",
        )
        val now = nowMs()
        nextEligibleAtMs.entries.removeAll { now >= it.value }
        if (runningKey == runKey || now < nextEligibleAtMs.getOrDefault(runKey, 0L)) {
            capture(provider.id)
            return
        }
        runningKey = runKey
        val runEpoch = ++epoch
        requestNext(
            provider = provider,
            runKey = runKey,
            runEpoch = runEpoch,
            pending = requirements(provider).toList(),
            retry = 0,
            delayMs = 0L,
        )
    }

    fun cancel() {
        epoch += 1
        runningKey = null
    }

    private fun requestNext(
        provider: WebChatProviderIdentity,
        runKey: RunKey,
        runEpoch: Int,
        pending: List<Requirement>,
        retry: Int,
        delayMs: Long,
    ) {
        scheduleAction(delayMs) {
            if (!isCurrent(runKey, runEpoch)) return@scheduleAction
            val port = consumerPort()
            if (port == null) {
                finish(runKey, success = false)
                return@scheduleAction
            }
            val state = port.state()
            interactionCache.capture(provider.id, state)
            val unresolved = pending.filterNot {
                it.isAvailable(provider.id, state, interactionCache)
            }
            val requirement = unresolved.firstOrNull()
            when {
                requirement == null -> settle(provider, runKey, runEpoch)
                requirement.request(port).accepted -> requestNext(
                    provider,
                    runKey,
                    runEpoch,
                    unresolved.drop(1),
                    retry = 0,
                    delayMs = REQUEST_SPACING_MS,
                )
                retry < RETRY_DELAYS_MS.size -> requestNext(
                    provider,
                    runKey,
                    runEpoch,
                    unresolved,
                    retry = retry + 1,
                    delayMs = RETRY_DELAYS_MS[retry],
                )
                else -> requestNext(
                    provider,
                    runKey,
                    runEpoch,
                    unresolved.drop(1),
                    retry = 0,
                    delayMs = REQUEST_SPACING_MS,
                )
            }
        }
    }

    private fun settle(
        provider: WebChatProviderIdentity,
        runKey: RunKey,
        runEpoch: Int,
    ) {
        SETTLE_DELAYS_MS.forEachIndexed { index, delayMs ->
            scheduleAction(delayMs) {
                if (!isCurrent(runKey, runEpoch)) return@scheduleAction
                capture(provider.id)
                if (index == SETTLE_DELAYS_MS.lastIndex) {
                    val state = consumerPort()?.state()
                    finish(
                        runKey,
                        success = state != null && requirements(provider).all {
                            it.isAvailable(provider.id, state, interactionCache)
                        },
                    )
                }
            }
        }
    }

    private fun capture(providerId: WebChatProviderId) {
        if (activeProvider() != providerId) return
        consumerPort()?.state()?.let { interactionCache.capture(providerId, it) }
    }

    private fun finish(runKey: RunKey, success: Boolean) {
        if (runningKey != runKey) return
        runningKey = null
        nextEligibleAtMs[runKey] = nowMs() + if (success) {
            SUCCESS_COOLDOWN_MS
        } else {
            FAILURE_COOLDOWN_MS
        }
    }

    private fun isCurrent(runKey: RunKey, runEpoch: Int): Boolean =
        epoch == runEpoch && runningKey == runKey && activeProvider() == runKey.providerId &&
            consumerPort()?.state()?.let(WebChatProductionPageIdentity::from)?.cacheKey == runKey.pageKey

    private data class RunKey(
        val providerId: WebChatProviderId,
        val pageKey: String,
    )

    private fun requirements(provider: WebChatProviderIdentity): Set<Requirement> = buildSet {
        if (provider.supports(WebChatProviderCapability.MODEL_SELECTOR)) add(Requirement.MODELS)
        if (provider.supports(WebChatProviderCapability.COMPOSER_TOOLS)) add(Requirement.TOOLS)
        if (provider.supports(WebChatProviderCapability.FEATURE_NAVIGATION)) add(Requirement.FEATURES)
        if (provider.supports(WebChatProviderCapability.PAGE_ACTIONS)) add(Requirement.CONTROLS)
    }

    private enum class Requirement {
        MODELS,
        TOOLS,
        FEATURES,
        CONTROLS;

        fun isAvailable(
            providerId: WebChatProviderId,
            state: WebChatConsumerState,
            cache: WebChatProductionInteractionCache,
        ): Boolean = when (this) {
            MODELS -> !cache.needsComposerRefresh(providerId, MODEL_SECTION)
            TOOLS -> !cache.needsComposerRefresh(providerId, TOOLS_SECTION)
            FEATURES -> !cache.needsFeatureRefresh(providerId)
            CONTROLS -> !cache.needsControlRefresh(providerId, state)
        }

        fun request(port: WebChatConsumerPort): WebChatConsumerCommandResult = when (this) {
            MODELS -> port.requestComposerOptions(MODEL_SECTION)
            TOOLS -> port.requestComposerOptions(TOOLS_SECTION)
            FEATURES -> port.requestFeatures()
            CONTROLS -> port.requestControls()
        }
    }

    private companion object {
        const val MODEL_SECTION = "model"
        const val TOOLS_SECTION = "tools"
        const val SUCCESS_COOLDOWN_MS = 60_000L
        const val FAILURE_COOLDOWN_MS = 5_000L
        const val REQUEST_SPACING_MS = 750L
        val RETRY_DELAYS_MS = longArrayOf(800L)
        val SETTLE_DELAYS_MS = longArrayOf(450L, 1_400L)
    }
}
