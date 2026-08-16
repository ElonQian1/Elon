package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProviderSwitchPolicyTest {
    @Test
    fun sameActiveProviderDoesNotPromptOrSwitch() {
        assertEquals(
            WebChatProviderSwitchDecision.ALREADY_ACTIVE,
            WebChatProviderSwitchPolicy.resolve(
                currentProvider = WebChatProviderId.CHATGPT_WEB,
                targetProvider = WebChatProviderId.CHATGPT_WEB,
                chatModeActive = true,
                pendingAttachmentCount = 2,
            ),
        )
    }

    @Test
    fun switchingWithPendingAttachmentsRequiresExplicitDiscard() {
        assertEquals(
            WebChatProviderSwitchDecision.CONFIRM_ATTACHMENT_DISCARD,
            WebChatProviderSwitchPolicy.resolve(
                currentProvider = WebChatProviderId.CHATGPT_WEB,
                targetProvider = WebChatProviderId.GOOGLE_WEB,
                chatModeActive = true,
                pendingAttachmentCount = 1,
            ),
        )
    }

    @Test
    fun switchingWithoutAttachmentsCanProceedImmediately() {
        assertEquals(
            WebChatProviderSwitchDecision.SWITCH_NOW,
            WebChatProviderSwitchPolicy.resolve(
                currentProvider = WebChatProviderId.CHATGPT_WEB,
                targetProvider = WebChatProviderId.GOOGLE_WEB,
                chatModeActive = true,
                pendingAttachmentCount = 0,
            ),
        )
    }
}
