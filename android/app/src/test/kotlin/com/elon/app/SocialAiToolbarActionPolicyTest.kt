package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SocialAiToolbarActionPolicyTest {
    @Test
    fun keepsTheVoiceShortcutOnlyInWorkMode() {
        assertTrue(SocialAiToolbarActionPolicy.showVoiceCall(
            directSocialAiChatActive = true,
            webChatModeActive = false,
        ))
        assertFalse(SocialAiToolbarActionPolicy.showVoiceCall(
            directSocialAiChatActive = true,
            webChatModeActive = true,
        ))
        assertFalse(SocialAiToolbarActionPolicy.showVoiceCall(
            directSocialAiChatActive = false,
            webChatModeActive = false,
        ))
    }
}
