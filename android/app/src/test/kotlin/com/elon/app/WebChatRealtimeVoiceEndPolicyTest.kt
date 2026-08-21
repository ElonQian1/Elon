package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatRealtimeVoiceEndPolicyTest {
    @Test
    fun resolvesVisibleOfficialVoiceHangUpControl() {
        val resolved = WebChatRealtimeVoiceEndPolicy.resolve(
            listOf(descriptor("control_end", "close", "Exit voice mode")),
        )

        assertEquals("control_end", resolved?.id)
    }

    @Test
    fun ignoresUnrelatedCloseControlsAndOffscreenVoiceControls() {
        assertNull(WebChatRealtimeVoiceEndPolicy.resolve(listOf(
            descriptor("dialog_close", "close", "Close dialog"),
            descriptor("voice_end", "close", "结束语音", inViewport = false),
        )))
    }

    private fun descriptor(
        id: String,
        semantic: String,
        label: String,
        inViewport: Boolean = true,
    ) = WebChatConsumerControlDescriptor(
        control = FakeControl(id, semantic, label, inViewport),
        requiresUserConfirmation = false,
        presentation = WebChatConsumerControlPresentation.DIRECT,
        nativeSelector = null,
    )

    private data class FakeControl(
        override val id: String,
        override val semantic: String,
        override val label: String,
        override val inViewport: Boolean,
    ) : WebChatConsumerControl {
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
        override val webXRatio: Double? = 0.5
        override val webYRatio: Double? = 0.5
    }
}
