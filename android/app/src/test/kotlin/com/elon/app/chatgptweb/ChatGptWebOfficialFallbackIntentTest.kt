package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebOfficialFallbackIntentTest {
    @Test
    fun acceptsCurrentChatGptConversationAndAuthenticationUrls() {
        assertEquals(
            "https://chatgpt.com/c/example",
            ChatGptWebOfficialFallbackIntent.sanitizeStartUrl(" https://chatgpt.com/c/example "),
        )
        assertEquals(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id=test",
            ChatGptWebOfficialFallbackIntent.sanitizeStartUrl(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id=test",
            ),
        )
        assertEquals(
            "https://chatgpt.com/auth/login",
            ChatGptWebOfficialFallbackIntent.sanitizeStartUrl(
                ChatGptWebOfficialFallbackIntent.LOGIN_URL,
            ),
        )
    }

    @Test
    fun rejectsUntrustedOrNonHttpsFallbackUrls() {
        assertNull(ChatGptWebOfficialFallbackIntent.sanitizeStartUrl("http://chatgpt.com/c/example"))
        assertNull(ChatGptWebOfficialFallbackIntent.sanitizeStartUrl("https://example.com/phishing"))
        assertNull(ChatGptWebOfficialFallbackIntent.sanitizeStartUrl("javascript:alert(1)"))
    }

    @Test
    fun parsesOnlyKnownOneShotStartupActions() {
        assertEquals(
            ChatGptWebOfficialStartupAction.REALTIME_VOICE,
            ChatGptWebOfficialStartupAction.fromWireValue(" realtime_voice "),
        )
        assertNull(ChatGptWebOfficialStartupAction.fromWireValue("unknown"))
        assertNull(ChatGptWebOfficialStartupAction.fromWireValue(null))
    }
}
