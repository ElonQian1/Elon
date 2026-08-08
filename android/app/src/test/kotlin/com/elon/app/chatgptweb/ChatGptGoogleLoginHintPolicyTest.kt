package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptGoogleLoginHintPolicyTest {
    @Test
    fun appendsEncodedLoginHintWithoutReencodingOAuthParameters() {
        val original = "https://accounts.google.com/o/oauth2/v2/auth" +
            "?client_id=client%3Avalue" +
            "&redirect_uri=https%3A%2F%2Fauth.openai.com%2Fcallback" +
            "&state=a%2Bb%2Fc#provider-fragment"

        val rewritten = ChatGptGoogleLoginHintPolicy.rewriteAuthorizationUrl(
            original,
            "first.last+chat@gmail.com",
        )

        requireNotNull(rewritten)
        assertTrue(rewritten.contains("client_id=client%3Avalue"))
        assertTrue(rewritten.contains("redirect_uri=https%3A%2F%2Fauth.openai.com%2Fcallback"))
        assertTrue(rewritten.contains("state=a%2Bb%2Fc"))
        assertTrue(rewritten.contains("login_hint=first.last%2Bchat%40gmail.com"))
        assertTrue(rewritten.endsWith("#provider-fragment"))
    }

    @Test
    fun doesNotOverrideAnExistingLoginHint() {
        val original = "https://accounts.google.com/o/oauth2/v2/auth?login_hint=chosen%40example.com"

        assertNull(
            ChatGptGoogleLoginHintPolicy.rewriteAuthorizationUrl(
                original,
                "another@gmail.com",
            ),
        )
    }

    @Test
    fun rejectsNonGoogleInsecureDeceptiveAndNonAuthorizationUrls() {
        listOf(
            "http://accounts.google.com/o/oauth2/v2/auth?client_id=x",
            "https://accounts.google.com.evil.example/o/oauth2/v2/auth?client_id=x",
            "https://user@accounts.google.com/o/oauth2/v2/auth?client_id=x",
            "https://accounts.google.com:8443/o/oauth2/v2/auth?client_id=x",
            "https://accounts.google.com/signin/v2/identifier?client_id=x",
            "https://chatgpt.com/auth/login",
        ).forEach { url ->
            assertNull(
                "expected no rewrite for $url",
                ChatGptGoogleLoginHintPolicy.rewriteAuthorizationUrl(url, "user@gmail.com"),
            )
        }
    }

    @Test
    fun normalizesOnlyBoundedEmailShapedAccountNames() {
        assertEquals(
            "user@gmail.com",
            ChatGptGoogleLoginHintPolicy.normalizeAccountName("  user@gmail.com "),
        )
        listOf(null, "", "google-user", "@gmail.com", "user@", "a @gmail.com", "a@@gmail.com")
            .forEach { accountName ->
                assertNull(ChatGptGoogleLoginHintPolicy.normalizeAccountName(accountName))
            }
    }
}
