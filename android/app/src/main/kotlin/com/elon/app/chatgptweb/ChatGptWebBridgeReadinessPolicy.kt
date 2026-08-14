package com.elon.app.chatgptweb

internal object ChatGptWebBridgeReadinessPolicy {
    fun stateAfterPageReady(
        listenerInstalled: Boolean,
        enhancedModeSupported: Boolean,
        document: ChatGptWebDocumentSession.Snapshot,
    ): ChatGptWebPageAdapter.State = when {
        !enhancedModeSupported -> ChatGptWebPageAdapter.State.WEB_ONLY
        !listenerInstalled -> ChatGptWebPageAdapter.State.UNSUPPORTED
        document.adapterCurrent -> ChatGptWebPageAdapter.State.READY
        else -> ChatGptWebPageAdapter.State.CONNECTING
    }

    fun canRestoreFromManifest(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest,
    ): Boolean = snapshot?.authenticated == true &&
        snapshot.loginRequired.not() &&
        manifest.pageKind != "auth"
}
