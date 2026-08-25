package com.elon.app

import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity

internal fun createWebChatRealtimeVoiceCoordinator(
    activity: AppCompatActivity,
    surface: WebChatRealtimeVoiceSurface,
    activeProvider: () -> WebChatProviderId?,
    consumerPort: () -> WebChatConsumerPort?,
    sessionReady: () -> Boolean,
    audioActivationEvidence: () -> WebChatRealtimeVoiceActivationEvidence,
    authenticated: () -> Boolean,
    sessionState: () -> String,
    beginWebBacking: () -> Boolean,
    endWebBacking: (Boolean) -> Unit,
    showInteractiveActivation: () -> Boolean,
    restoreNativeSurface: () -> Unit,
    requestSessionRecovery: () -> Unit,
    openOfficialLogin: () -> Unit,
    openOfficialFallback: () -> Unit,
    resolveConversationContext: () -> WebChatRealtimeVoiceContext,
    openConversation: (WebChatRealtimeVoiceContext) -> Unit,
    launchCache: WebChatRealtimeVoiceLaunchCache = WebChatRealtimeVoiceLaunchCache(),
): WebChatRealtimeVoiceCoordinator {
    lateinit var coordinator: WebChatRealtimeVoiceCoordinator
    val backgroundBridge = WebChatRealtimeVoiceBackgroundBridge(activity)
    val callback = object : OnBackPressedCallback(false) {
        override fun handleOnBackPressed() = coordinator.close()
    }
    activity.onBackPressedDispatcher.addCallback(activity, callback)
    coordinator = WebChatRealtimeVoiceCoordinator(
        surface = surface,
        activeProvider = activeProvider,
        consumerPort = consumerPort,
        sessionReady = sessionReady,
        audioActivationEvidence = audioActivationEvidence,
        authenticationState = {
            WebChatRealtimeVoiceAuthenticationPolicy.resolve(authenticated(), sessionState())
        },
        beginWebBacking = beginWebBacking,
        endWebBacking = endWebBacking,
        showInteractiveActivation = showInteractiveActivation,
        restoreNativeSurface = restoreNativeSurface,
        requestSessionRecovery = requestSessionRecovery,
        loginGate = WebChatRealtimeVoiceLoginDialog(activity),
        openOfficialLogin = openOfficialLogin,
        openOfficialFallback = openOfficialFallback,
        resolveConversationContext = resolveConversationContext,
        openConversation = openConversation,
        schedule = { task, delayMs -> activity.window.decorView.postDelayed(task, delayMs) },
        backControl = object : WebChatRealtimeVoiceBackControl {
            override fun setEnabled(enabled: Boolean) {
                callback.isEnabled = enabled
            }

            override fun dispose() = callback.remove()
        },
        backgroundBridge = backgroundBridge,
        launchCache = launchCache,
    )
    backgroundBridge.attach(coordinator)
    return coordinator
}
