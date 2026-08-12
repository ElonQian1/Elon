package com.elon.app.chatgptweb

internal class ChatGptWebUiControlInvocationCoordinator(
    private val isOfficialVisible: () -> Boolean,
    private val showOfficial: () -> Unit,
    private val schedule: (Long, () -> Unit) -> Unit,
    private val beginDictation: ((() -> Unit) -> Long?),
    private val onDictationTimedOut: (Long) -> Unit,
    private val startOfficialDictation: (String?) -> Unit,
    private val failCommand: (String, String, String) -> Unit,
    private val invokeOfficialControl: (String, String?) -> Unit,
) {
    private var generation = 0

    fun invoke(control: ChatGptWebUiControl?, controlId: String, requestId: String?) {
        if (control?.semantic == ChatGptWebUiSemantics.DICTATION) {
            prepareDictation(requestId, "invoke_ui_control") {
                invokeOfficialControl(controlId, requestId)
            }
            return
        }
        if (!requiresOfficialLayout(control) || isOfficialVisible()) {
            invokeOfficialControl(controlId, requestId)
            return
        }

        val requestGeneration = ++generation
        showOfficial()
        schedule(OFFICIAL_LAYOUT_SETTLE_MS) {
            if (requestGeneration == generation) {
                invokeOfficialControl(controlId, requestId)
            }
        }
    }

    fun startDictation(requestId: String? = null) {
        prepareDictation(requestId, "start_dictation") {
            startOfficialDictation(requestId)
        }
    }

    fun dispose() {
        generation += 1
    }

    private fun prepareDictation(
        requestId: String?,
        expectedAction: String,
        startOfficial: () -> Unit,
    ) {
        val attempt = beginDictation(startOfficial)
        if (attempt == null) {
            requestId?.let {
                failCommand(it, expectedAction, "dictation_start_in_progress")
            }
            return
        }
        schedule(DICTATION_START_TIMEOUT_MS) {
            onDictationTimedOut(attempt)
        }
    }

    internal companion object {
        const val OFFICIAL_LAYOUT_SETTLE_MS = 320L
        const val DICTATION_START_TIMEOUT_MS = 20_000L

        fun requiresOfficialLayout(control: ChatGptWebUiControl?): Boolean = when {
            control?.semantic == ChatGptRealtimeVoicePolicy.SEMANTIC -> true
            control?.region == ChatGptWebUiRegion.MESSAGE && control.semantic == "more" -> true
            else -> false
        }
    }
}
