package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebNavigationPolicyTest {
    @Test
    fun allowsChatGptOpenAiAndSupportedIdentityHosts() {
        listOf(
            "https://chatgpt.com/",
            ChatGptWebNavigationPolicy.AUTH_URL,
            "https://auth.openai.com/log-in",
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://login.microsoftonline.com/common/oauth2/authorize",
            "https://login.live.com/oauth20_authorize.srf",
            "https://appleid.apple.com/auth/authorize",
        ).forEach { url -> assertTrue("expected allowed URL: $url", ChatGptWebNavigationPolicy.allows(url)) }
    }

    @Test
    fun blocksInsecureDeceptiveAndUnrelatedUrls() {
        listOf(
            "http://chatgpt.com/",
            "https://chatgpt.com.evil.example/",
            "https://evil.example/openai.com",
            "https://user@chatgpt.com/",
            "https://chatgpt.com:8443/",
            "intent://chatgpt.com/",
            "javascript:alert(1)",
        ).forEach { url -> assertFalse("expected blocked URL: $url", ChatGptWebNavigationPolicy.allows(url)) }
    }

    @Test
    fun enhancedModeOnlyRunsOnTheExactChatGptOrigin() {
        assertTrue(ChatGptWebNavigationPolicy.supportsEnhancedMode("https://chatgpt.com/c/123"))
        listOf(
            "https://auth.openai.com/log-in",
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://sub.chatgpt.com/",
            "http://chatgpt.com/",
            "https://chatgpt.com.evil.example/",
        ).forEach { url ->
            assertFalse("enhanced mode must reject: $url", ChatGptWebNavigationPolicy.supportsEnhancedMode(url))
        }
    }

    @Test
    fun authenticationPageDetectionIsExactOriginAndAuthPathOnly() {
        assertTrue(ChatGptWebNavigationPolicy.isAuthenticationPage("https://chatgpt.com/auth/login"))
        assertTrue(ChatGptWebNavigationPolicy.isAuthenticationPage("https://chatgpt.com/auth/error"))
        listOf(
            "https://chatgpt.com/",
            "https://sub.chatgpt.com/auth/login",
            "https://chatgpt.com.evil.example/auth/login",
            "http://chatgpt.com/auth/login",
        ).forEach { url ->
            assertFalse("authentication page must reject: $url", ChatGptWebNavigationPolicy.isAuthenticationPage(url))
        }
    }
}
