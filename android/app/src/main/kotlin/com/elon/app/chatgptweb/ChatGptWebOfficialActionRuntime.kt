package com.elon.app.chatgptweb

import android.content.Context
import android.webkit.WebView
import java.util.UUID

internal class ChatGptWebOfficialActionRuntime(
    context: Context,
    private val webView: WebView,
    startupAction: ChatGptWebOfficialStartupAction?,
    audioPermissionController: ChatGptWebAudioPermissionController,
    onFeedback: (ChatGptWebOfficialStartupFeedback) -> Unit,
) {
    private val touchDispatcher = ChatGptWebTouchDispatcher(webView)
    private lateinit var adapter: ChatGptWebPageAdapter
    private val coordinator = ChatGptWebOfficialStartupCoordinator(
        action = startupAction,
        requestManifest = { adapter.requestUiManifest() },
        requestMicrophone = { onGranted, onDenied ->
            audioPermissionController.runWithMicrophone(onGranted, onDenied)
        },
        invokeControl = { controlId, requestId ->
            adapter.invokeUiControl(controlId, requestId)
        },
        schedule = { delayMs, action -> webView.postDelayed(action, delayMs) },
        requestId = { "official-voice-${UUID.randomUUID()}" },
        onFeedback = onFeedback,
    )

    init {
        adapter = ChatGptWebPageAdapter(
            context = context,
            webView = webView,
            onEvent = ::onEvent,
            onStateChanged = { state ->
                if (state == ChatGptWebPageAdapter.State.READY) coordinator.onAdapterReady()
            },
        )
        adapter.install()
    }

    fun onPageStarted(url: String) {
        coordinator.onPageStarted()
        adapter.onPageStarted(url)
    }

    fun onPageReady(url: String) {
        adapter.onPageReady(url)
        coordinator.onPageReady(ChatGptWebNavigationPolicy.supportsEnhancedMode(url))
    }

    fun onHostResumed() {
        adapter.onHostResumed(webView.url)
        coordinator.onHostResumed()
    }

    fun onHostPaused() {
        coordinator.onHostPaused()
        adapter.onHostPaused()
    }

    fun requestConsumed(): Boolean = coordinator.requestConsumed()

    fun dispose() = coordinator.dispose()

    private fun onEvent(event: ChatGptWebEvent) {
        coordinator.onEvent(event)
        if (event is ChatGptWebEvent.WebTouchRequest) {
            touchDispatcher.dispatch(event) { dispatched ->
                if (!dispatched) coordinator.onTouchDispatchFailed(event.controlId)
            }
        }
    }
}
