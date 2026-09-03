package com.elon.app

internal enum class WebChatDictationTransport {
    PRIVATE,
    SHARED,
    DOM,
}

internal enum class WebChatProductionDictationTapRoute {
    SUBMIT_PRIVATE,
    SUBMIT_SHARED,
    SUBMIT_DOM,
    START,
    NONE,
}

internal object WebChatProductionDictationRoutePolicy {
    fun resolve(
        privateActive: Boolean,
        sharedActive: Boolean,
        domActive: Boolean,
        startAvailable: Boolean,
    ): WebChatProductionDictationTapRoute = when {
        privateActive -> WebChatProductionDictationTapRoute.SUBMIT_PRIVATE
        sharedActive -> WebChatProductionDictationTapRoute.SUBMIT_SHARED
        domActive -> WebChatProductionDictationTapRoute.SUBMIT_DOM
        startAvailable -> WebChatProductionDictationTapRoute.START
        else -> WebChatProductionDictationTapRoute.NONE
    }
}

internal interface WebChatPrivateDictationPort {
    fun ready(): Boolean
    fun start(
        onStateChanged: (WebChatNativeDictationState) -> Unit,
        onUnavailableBeforeCapture: () -> Unit,
    ): Boolean
    fun submit(): Boolean
    fun cancel(): Boolean
    fun state(): WebChatNativeDictationState
    fun onCommandResult(action: String, ok: Boolean, detail: String) = Unit
    fun observeOfficialDraft(draft: String) = Unit
    fun destroy()
}

internal object WebChatUnavailablePrivateDictationPort : WebChatPrivateDictationPort {
    override fun ready(): Boolean = false

    override fun start(
        onStateChanged: (WebChatNativeDictationState) -> Unit,
        onUnavailableBeforeCapture: () -> Unit,
    ): Boolean = false

    override fun submit(): Boolean = false

    override fun cancel(): Boolean = false

    override fun state(): WebChatNativeDictationState = WebChatNativeDictationState()

    override fun destroy() = Unit
}

internal object WebChatDictationStartChain {
    fun start(
        privateReady: Boolean,
        startPrivate: () -> Boolean,
        startShared: () -> Boolean,
        startDom: () -> Boolean,
    ): WebChatDictationTransport? {
        if (privateReady && startPrivate()) return WebChatDictationTransport.PRIVATE
        if (startShared()) return WebChatDictationTransport.SHARED
        if (startDom()) return WebChatDictationTransport.DOM
        return null
    }
}
