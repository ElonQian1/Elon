package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptRealtimeVoicePolicyTest {
    @Test
    fun resolvesOnlyAnEnabledRealtimeVoiceControl() {
        assertNull(ChatGptRealtimeVoicePolicy.resolve(null))
        assertNull(ChatGptRealtimeVoicePolicy.resolve(manifest(control("other", enabled = true))))
        assertNull(ChatGptRealtimeVoicePolicy.resolve(manifest(control("voice_mode", enabled = false))))

        assertEquals(
            "control_voice",
            ChatGptRealtimeVoicePolicy.resolve(
                manifest(control("voice_mode", enabled = true)),
            )?.id,
        )
    }

    private fun manifest(control: ChatGptWebUiControl) = ChatGptWebUiManifest(
        version = 3,
        pageKind = "conversation",
        title = "ChatGPT",
        compatibility = "healthy",
        controls = listOf(control),
    )

    private fun control(semantic: String, enabled: Boolean) = ChatGptWebUiControl(
        id = "control_voice",
        semantic = semantic,
        label = "实时语音",
        region = ChatGptWebUiRegion.COMPOSER,
        role = "button",
        enabled = enabled,
        selected = false,
    )
}
