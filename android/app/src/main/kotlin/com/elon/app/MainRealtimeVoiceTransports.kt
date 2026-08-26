package com.elon.app

import android.view.View
import androidx.appcompat.app.AppCompatActivity

/** Uses official WebRTC by default while retaining the server API path as a disabled experiment. */
internal class MainRealtimeVoiceTransports(
    activity: AppCompatActivity,
    controller: () -> ChatGptSocialChatController,
    webLifecycle: MainChatGptWebLifecycle,
    modeController: SocialAiChatModeController,
    nativeRoot: View,
    activeProvider: () -> WebChatProviderId?,
    serverUrl: () -> String,
    userId: () -> String,
    launchCache: WebChatRealtimeVoiceLaunchCache,
) {
    private val webDelegate = lazy {
        createMainRealtimeVoiceCoordinator(
            activity = activity,
            activeProvider = activeProvider,
            controller = controller(),
            webLifecycle = webLifecycle,
            modeController = modeController,
            nativeRoot = nativeRoot,
            launchCache = launchCache,
        )
    }
    private val web by webDelegate
    private val nativeDelegate = lazy {
        NativeApiRealtimeVoiceCoordinator(
            activity = activity,
            surface = WebChatRealtimeVoiceOverlay(activity),
            audioPermissionController = webLifecycle.audioPermissionController,
            serverUrl = serverUrl,
            userId = userId,
            openLocalConversation = {
                modeController.selectInteractionMode(SocialAiInteractionMode.WORK)
                Unit
            },
            openOfficialFallback = {
                modeController.openOfficialRealtimeVoice()
                Unit
            },
        )
    }
    private val native by nativeDelegate

    fun startDefaultOfficialWebRtc(provider: WebChatProviderIdentity): Boolean =
        RealtimeVoiceTransportCatalog.officialWebRtc.runtimeEnabled &&
            (!nativeDelegate.isInitialized() || !native.isActive()) && web.start(provider)

    fun startServerApiExperiment(): Boolean =
        RealtimeVoiceTransportCatalog.serverApiExperiment.runtimeEnabled &&
            (!webDelegate.isInitialized() || !web.isActive()) && native.start()

    fun onConsumerStateChanged(state: WebChatConsumerState) = web.onConsumerStateChanged(state)

    fun onHostResumed() {
        if (webDelegate.isInitialized()) web.onHostResumed()
        if (nativeDelegate.isInitialized()) native.onHostResumed()
    }

    fun onHostPaused() {
        if (webDelegate.isInitialized()) web.onHostPaused()
        if (nativeDelegate.isInitialized()) native.onHostPaused()
    }

    fun onActiveSurfaceChanged() {
        if (webDelegate.isInitialized()) web.onActiveSurfaceChanged()
    }

    fun destroy() {
        if (webDelegate.isInitialized()) web.destroy()
        if (nativeDelegate.isInitialized()) native.destroy()
    }
}
