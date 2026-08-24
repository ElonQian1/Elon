package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAccessPolicyTest {
    @Test
    fun anonymousComposerCanChatWithoutPretendingToBeAuthenticated() {
        val anonymous = snapshot(authenticated = false, composerReady = true)

        assertTrue(ChatGptWebAccessPolicy.canChat(anonymous))
        assertFalse(ChatGptWebAccessPolicy.requiresLogin(anonymous))
    }

    @Test
    fun authenticatedComposerCanChat() {
        assertTrue(ChatGptWebAccessPolicy.canChat(snapshot(authenticated = true, composerReady = true)))
    }

    @Test
    fun authPageStillRequiresLoginEvenIfAFalseComposerIsObserved() {
        val auth = snapshot(
            authenticated = false,
            composerReady = true,
            loginRequired = true,
            pageKind = "auth",
        )

        assertTrue(ChatGptWebAccessPolicy.requiresLogin(auth))
        assertFalse(ChatGptWebAccessPolicy.canChat(auth))
    }

    @Test
    fun rateLimitedResponseCannotReuseAStillVisibleComposer() {
        assertFalse(
            ChatGptWebAccessPolicy.canChat(
                snapshot(authenticated = false, composerReady = true, accessReason = "rate_limited"),
            ),
        )
    }

    @Test
    fun privateAuthenticationHintRequiresLoginWhileVisiblePageCatchesUp() {
        assertTrue(
            ChatGptWebAccessPolicy.requiresLogin(
                snapshot(authenticated = false, composerReady = false, accessReason = "login_required"),
            ),
        )
    }

    private fun snapshot(
        authenticated: Boolean,
        composerReady: Boolean,
        loginRequired: Boolean = false,
        pageKind: String = "conversation",
        accessReason: String = "",
    ) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        authenticated = authenticated,
        composerReady = composerReady,
        streaming = false,
        currentModel = "",
        messages = emptyList(),
        draft = "",
        capabilities = ChatGptWebCapabilities.EMPTY,
        attachments = emptyList(),
        dictationActive = false,
        pageKind = pageKind,
        loginRequired = loginRequired,
        accessReason = accessReason,
    )
}
