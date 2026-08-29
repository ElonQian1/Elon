package com.elon.app

import android.view.View
import androidx.appcompat.app.AppCompatActivity

internal fun createMainRealtimeVoiceCoordinator(
    activity: AppCompatActivity,
    activeProvider: () -> WebChatProviderId?,
    controller: ChatGptSocialChatController,
    webLifecycle: MainChatGptWebLifecycle,
    modeController: SocialAiChatModeController,
    nativeRoot: View,
    launchCache: WebChatRealtimeVoiceLaunchCache,
    surface: WebChatRealtimeVoiceSurface = WebChatRealtimeVoiceOverlay(activity),
): WebChatRealtimeVoiceCoordinator = createWebChatRealtimeVoiceCoordinator(
    activity = activity,
    surface = surface,
    activeProvider = activeProvider,
    consumerPort = controller::consumerPort,
    sessionReady = { controller.stateWireValue() == "ready" && controller.composerReady() },
    audioActivationEvidence = webLifecycle::realtimeVoiceActivationEvidence,
    authenticated = controller::authenticated,
    sessionState = controller::stateWireValue,
    beginWebBacking = controller::beginRealtimeVoiceBacking,
    endWebBacking = controller::endRealtimeVoiceBacking,
    startManagedWebRtc = controller::startManagedRealtimeVoice,
    managedWebRtcState = controller::managedRealtimeVoiceState,
    setManagedWebRtcMuted = controller::setManagedRealtimeVoiceMuted,
    showInteractiveActivation = controller::showWebSkin,
    restoreNativeSurface = { controller.showNativeMirror(); Unit },
    requestSessionRecovery = controller::onHostResumed,
    openOfficialLogin = modeController::openOfficialLogin,
    openOfficialFallback = modeController::openOfficialRealtimeVoice,
    resolveConversationContext = { resolveRealtimeVoiceContext(controller) },
    openConversation = {
        openRealtimeVoiceConversation(it, modeController, nativeRoot, controller)
    },
    launchCache = launchCache,
)
