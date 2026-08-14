package com.elon.app

internal enum class WebBridgeConnectionState {
    WEB_ONLY,
    CONNECTING,
    READY,
    UNSUPPORTED,
}

internal object WebBridgeReadinessPolicy {
    fun stateAfterPageReady(
        listenerInstalled: Boolean,
        pageSupported: Boolean,
        document: WebBridgeDocumentSession.Snapshot,
    ): WebBridgeConnectionState = when {
        !pageSupported -> WebBridgeConnectionState.WEB_ONLY
        !listenerInstalled -> WebBridgeConnectionState.UNSUPPORTED
        document.adapterCurrent -> WebBridgeConnectionState.READY
        else -> WebBridgeConnectionState.CONNECTING
    }
}
