package com.elon.app.chatgptweb

import com.elon.app.WebBridgeConnectionState
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebBridgeReadinessPolicy

internal object ChatGptWebBridgeReadinessPolicy {
    fun stateAfterPageReady(
        listenerInstalled: Boolean,
        enhancedModeSupported: Boolean,
        document: WebBridgeDocumentSession.Snapshot,
    ): ChatGptWebPageAdapter.State = when (WebBridgeReadinessPolicy.stateAfterPageReady(
        listenerInstalled = listenerInstalled,
        pageSupported = enhancedModeSupported,
        document = document,
    )) {
        WebBridgeConnectionState.WEB_ONLY -> ChatGptWebPageAdapter.State.WEB_ONLY
        WebBridgeConnectionState.CONNECTING -> ChatGptWebPageAdapter.State.CONNECTING
        WebBridgeConnectionState.READY -> ChatGptWebPageAdapter.State.READY
        WebBridgeConnectionState.UNSUPPORTED -> ChatGptWebPageAdapter.State.UNSUPPORTED
    }

    fun canRestoreFromManifest(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest,
    ): Boolean = snapshot?.let(ChatGptWebAccessPolicy::canChat) == true &&
        manifest.pageKind != "auth"
}
