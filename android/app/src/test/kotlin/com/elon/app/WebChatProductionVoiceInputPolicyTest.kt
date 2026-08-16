package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionVoiceInputPolicyTest {
    @Test
    fun webChatAlwaysTranscribesIntoTheCurrentProvider() {
        assertEquals(
            WebChatProductionVoiceInputRoute.WEB_TEXT_TRANSCRIPTION,
            WebChatProductionVoiceInputPolicy.resolve(webChatModeActive = true, friendChatActive = true),
        )
        assertFalse(WebChatProductionVoiceInputPolicy.allowsDirectCloudAiFallback(true))
    }

    @Test
    fun ordinaryFriendChatKeepsVoiceMessages() {
        assertEquals(
            WebChatProductionVoiceInputRoute.VOICE_MESSAGE,
            WebChatProductionVoiceInputPolicy.resolve(webChatModeActive = false, friendChatActive = true),
        )
        assertTrue(WebChatProductionVoiceInputPolicy.allowsDirectCloudAiFallback(false))
    }

    @Test
    fun projectAndWorkSurfacesKeepTheirConfiguredVoiceRoute() {
        assertEquals(
            WebChatProductionVoiceInputRoute.CONFIGURED_WORK_INPUT,
            WebChatProductionVoiceInputPolicy.resolve(webChatModeActive = false, friendChatActive = false),
        )
    }
}
