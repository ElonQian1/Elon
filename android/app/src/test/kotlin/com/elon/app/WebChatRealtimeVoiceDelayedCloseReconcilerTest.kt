package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatRealtimeVoiceDelayedCloseReconcilerTest {
    @Test
    fun completesOnlyAfterStableConversationEvidence() {
        val reconciler = WebChatRealtimeVoiceDelayedCloseReconciler(
            maxPolls = 8,
            stableConversationPolls = 2,
            controlRefreshInterval = 2,
        )
        reconciler.begin()

        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = true),
            reconciler.observe(state(endControlAvailable = false)),
        )
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Complete,
            reconciler.observe(state(endControlAvailable = false)),
        )
    }

    @Test
    fun activeVoiceEvidenceNeverCompletesAndEventuallyExpires() {
        val reconciler = WebChatRealtimeVoiceDelayedCloseReconciler(
            maxPolls = 2,
            stableConversationPolls = 2,
            controlRefreshInterval = 2,
        )
        reconciler.begin()

        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = true),
            reconciler.observe(state(endControlAvailable = true)),
        )
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = true),
            reconciler.observe(state(endControlAvailable = true)),
        )
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Expired,
            reconciler.observe(state(endControlAvailable = true)),
        )
    }

    private fun state(endControlAvailable: Boolean) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = "conversation",
        pageUrl = "https://chatgpt.com/c/test",
        features = emptyList(),
        controls = if (endControlAvailable) {
            listOf(
                WebChatConsumerControlDescriptor(
                    control = VoiceEndControl,
                    requiresUserConfirmation = true,
                    presentation = WebChatConsumerControlPresentation.DIRECT,
                    nativeSelector = "voice_close",
                ),
            )
        } else {
            emptyList()
        },
        commandRequests = emptyList(),
    )

    private object VoiceEndControl : WebChatConsumerControl {
        override val id = "voice_end"
        override val semantic = "close"
        override val label = "结束语音"
        override val region = "voice"
        override val role = "button"
        override val enabled = true
        override val selected = false
        override val inputKind: String? = null
        override val writable = false
        override val stateSettable = false
        override val choiceLabels = emptyList<String>()
        override val selectedChoiceIndex: Int? = null
        override val slider: WebChatConsumerSlider? = null
        override val expanded: Boolean? = null
        override val expandable = false
        override val contextId: String? = null
        override val inViewport = true
        override val webXRatio: Double? = null
        override val webYRatio: Double? = null
    }
}
