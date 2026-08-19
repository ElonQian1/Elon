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
    private var runningProvider: WebChatProviderId? = null
    private val nextEligibleAtMs = mutableMapOf<WebChatProviderId, Long>()

    fun schedule(provider: WebChatProviderIdentity) {
        if (requirements(provider).isEmpty()) return
        val now = nowMs()
        if (runningProvider == provider.id || now < nextEligibleAtMs.getOrDefault(provider.id, 0L)) {
            capture(provider.id)
            return
        }
        runningProvider = provider.id
        val runEpoch = ++epoch
        requestNext(
            provider = provider,
            runEpoch = runEpoch,
            pending = requirements(provider).toList(),
            retry = 0,
            delayMs = 0L,
        )
    }

    fun cancel() {
        epoch += 1
        runningProvider = null
    }

    private fun requestNext(
        provider: WebChatProviderIdentity,
        runEpoch: Int,
        pending: List<Requirement>,
        retry: Int,
        delayMs: Long,
    ) {
        scheduleAction(delayMs) {
            if (!isCurrent(provider.id, runEpoch)) return@scheduleAction
            val port = consumerPort()
            if (port == null) {
                finish(provider.id, success = false)
                return@scheduleAction
            }
            val state = port.state()
            interactionCache.capture(provider.id, state)
            val unresolved = pending.filterNot {
                it.isAvailable(provider.id, state, interactionCache)
            }
            val requirement = unresolved.firstOrNull()
            when {
                requirement == null -> settle(provider, runEpoch)
                requirement.request(port).accepted -> requestNext(
                    provider,
                    runEpoch,
                    unresolved.drop(1),
                    retry = 0,
                    delayMs = REQUEST_SPACING_MS,
                )
                retry < RETRY_DELAYS_MS.size -> requestNext(
                    provider,
                    runEpoch,
                    unresolved,
                    retry = retry + 1,
                    delayMs = RETRY_DELAYS_MS[retry],
                )
                else -> requestNext(
                    provider,
                    runEpoch,
                    unresolved.drop(1),
                    retry = 0,
                    delayMs = REQUEST_SPACING_MS,
                )
            }
        }
    }

    private fun settle(provider: WebChatProviderIdentity, runEpoch: Int) {
        SETTLE_DELAYS_MS.forEachIndexed { index, delayMs ->
            scheduleAction(delayMs) {
                if (!isCurrent(provider.id, runEpoch)) return@scheduleAction
                capture(provider.id)
                if (index == SETTLE_DELAYS_MS.lastIndex) {
                    val state = consumerPort()?.state()
                    finish(
                        provider.id,
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

    private fun finish(providerId: WebChatProviderId, success: Boolean) {
        if (runningProvider != providerId) return
        runningProvider = null
        nextEligibleAtMs[providerId] = nowMs() + if (success) {
            SUCCESS_COOLDOWN_MS
        } else {
            FAILURE_COOLDOWN_MS
        }
    }

    private fun isCurrent(providerId: WebChatProviderId, runEpoch: Int): Boolean =
        epoch == runEpoch && runningProvider == providerId && activeProvider() == providerId

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
            MODELS -> state.composerSections[MODEL_SECTION].orEmpty().isNotEmpty() ||
                cache.hasComposerSnapshot(providerId, MODEL_SECTION)
            TOOLS -> state.composerSections[TOOLS_SECTION].orEmpty().isNotEmpty() ||
                cache.hasComposerSnapshot(providerId, TOOLS_SECTION)
            FEATURES -> state.features.isNotEmpty() || cache.hasFeatureSnapshot(providerId)
            CONTROLS -> state.controls.isNotEmpty() || cache.hasControlSnapshot(providerId, state)
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
