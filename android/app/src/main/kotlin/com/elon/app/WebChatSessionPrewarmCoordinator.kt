package com.elon.app

import android.view.View

internal class WebChatSessionPrewarmCoordinator(
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val interactionMode: () -> SocialAiInteractionMode,
    private val selectedProvider: () -> WebChatProviderId,
    private val hasWarmSession: (WebChatProviderId) -> Boolean,
    private val isProviderActive: (WebChatProviderId) -> Boolean,
    private val prewarm: (WebChatProviderId) -> Boolean,
    private val delayMs: Long = DEFAULT_DELAY_MS,
) {
    private var pending: Runnable? = null

    fun onHostResumed() {
        cancel()
        if (interactionMode() != SocialAiInteractionMode.CHAT) return
        val provider = selectedProvider()
        if (isProviderActive(provider) || !hasWarmSession(provider)) return

        lateinit var task: Runnable
        task = Runnable {
            if (pending !== task) return@Runnable
            pending = null
            if (interactionMode() != SocialAiInteractionMode.CHAT) return@Runnable
            val currentProvider = selectedProvider()
            if (!isProviderActive(currentProvider) && hasWarmSession(currentProvider)) {
                prewarm(currentProvider)
            }
        }
        pending = task
        schedule(task, delayMs)
    }

    fun cancel() {
        pending?.let(cancel)
        pending = null
    }

    private companion object {
        const val DEFAULT_DELAY_MS = 250L
    }
}

internal fun createWebChatSessionPrewarmCoordinator(
    host: View,
    modeController: SocialAiChatModeController,
    controllerFor: (WebChatProviderId) -> WebChatSocialController,
): WebChatSessionPrewarmCoordinator = WebChatSessionPrewarmCoordinator(
    schedule = { task, delayMs -> host.postDelayed(task, delayMs) },
    cancel = host::removeCallbacks,
    interactionMode = modeController::interactionMode,
    selectedProvider = modeController::providerId,
    hasWarmSession = { controllerFor(it).warmSessionAvailable() },
    isProviderActive = { controllerFor(it).isActive() },
    prewarm = { controllerFor(it).prewarm() },
)
