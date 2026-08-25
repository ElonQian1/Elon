package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceActivationEvidencePolicyTest {
    @Test
    fun acceptsOnlyACurrentOfficialVoiceSurface() {
        assertTrue(resolve(adapterCurrent = true).officialVoiceActive)
        assertFalse(resolve(adapterCurrent = false).officialVoiceActive)
    }

    private fun resolve(adapterCurrent: Boolean) =
        WebChatRealtimeVoiceActivationEvidencePolicy.resolve(
            permission = WebChatRealtimeVoiceActivationEvidence(
                androidPermissionGranted = true,
                webPermissionGrantRevision = 4,
                webRequestPending = false,
                requestState = "web_permission_granted",
            ),
            state = WebChatConsumerState(
                streaming = false,
                dictationActive = false,
                composerSections = emptyMap(),
                pageKind = "voice",
                pageUrl = "https://chatgpt.com/",
                features = emptyList(),
                controls = listOf(hangupDescriptor()),
                commandRequests = emptyList(),
                adapterCurrent = adapterCurrent,
            ),
        )

    private fun hangupDescriptor() = WebChatConsumerControlDescriptor(
        control = HangupControl,
        requiresUserConfirmation = false,
        presentation = WebChatConsumerControlPresentation.DIRECT,
        nativeSelector = null,
    )

    private object HangupControl : WebChatConsumerControl {
        override val id = "voice_end"
        override val semantic = "close"
        override val label = "结束语音"
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
