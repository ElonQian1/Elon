package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatRealtimeVoicePauseControlPolicyTest {
    @Test
    fun invokesMuteAndTreatsUnmuteAsAlreadyPaused() {
        assertEquals(
            WebChatRealtimeVoicePauseControlDecision.Invoke("mute"),
            WebChatRealtimeVoicePauseControlPolicy.decide(listOf(control("mute", "voice_mute")), true),
        )
        assertEquals(
            WebChatRealtimeVoicePauseControlDecision.AlreadyApplied,
            WebChatRealtimeVoicePauseControlPolicy.decide(listOf(control("unmute", "voice_unmute")), true),
        )
    }

    @Test
    fun invokesUnmuteAndTreatsMuteAsAlreadyResumed() {
        assertEquals(
            WebChatRealtimeVoicePauseControlDecision.Invoke("unmute"),
            WebChatRealtimeVoicePauseControlPolicy.decide(listOf(control("unmute", "voice_unmute")), false),
        )
        assertEquals(
            WebChatRealtimeVoicePauseControlDecision.AlreadyApplied,
            WebChatRealtimeVoicePauseControlPolicy.decide(listOf(control("mute", "voice_mute")), false),
        )
    }

    @Test
    fun refreshesWhenTheOfficialVoiceControlsHaveNotBeenObservedYet() {
        assertEquals(
            WebChatRealtimeVoicePauseControlDecision.RefreshControls,
            WebChatRealtimeVoicePauseControlPolicy.decide(emptyList(), true),
        )
    }

    private fun control(id: String, semantic: String) = WebChatConsumerControlDescriptor(
        control = FakeControl(id, semantic),
        requiresUserConfirmation = false,
        presentation = WebChatConsumerControlPresentation.DIRECT,
        nativeSelector = null,
    )

    private data class FakeControl(
        override val id: String,
        override val semantic: String,
    ) : WebChatConsumerControl {
        override val label = semantic
        override val region = "overlay"
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
        override val webXRatio: Double? = 0.5
        override val webYRatio: Double? = 0.5
    }
}
