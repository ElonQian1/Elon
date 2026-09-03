package com.elon.app

internal enum class WebChatDictationMode {
    PRIVATE,
    SHARED,
    ;

    fun toggled(): WebChatDictationMode = when (this) {
        PRIVATE -> SHARED
        SHARED -> PRIVATE
    }
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

internal class WebChatDictationModeSelector(
    initialMode: WebChatDictationMode = WebChatDictationMode.PRIVATE,
) {
    var selected: WebChatDictationMode = initialMode
        private set

    fun toggle(): WebChatDictationMode {
        selected = selected.toggled()
        return selected
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

internal class WebChatDictationRearmGate(
    private val clock: () -> Long,
    private val settleMs: Long = DEFAULT_SETTLE_MS,
) {
    private var observedActive = false
    private var blockedUntilMs = 0L

    fun observe(active: Boolean): Boolean {
        val completed = observedActive && !active
        observedActive = active
        if (completed) blockedUntilMs = clock() + settleMs
        return completed
    }

    fun canStart(): Boolean = clock() >= blockedUntilMs

    fun remainingMs(): Long = (blockedUntilMs - clock()).coerceAtLeast(0L)

    private companion object {
        const val DEFAULT_SETTLE_MS = 600L
    }
}
