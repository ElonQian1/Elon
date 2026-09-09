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

    @Test
    fun projectWithoutAComposerCanReadItsDirectoryAndNavigateButCannotSend() {
        val project = snapshot(authenticated = true, composerReady = false, pageKind = "feature")
            .copy(url = "https://chatgpt.com/g/g-p-project/project")

        assertTrue(ChatGptWebAccessPolicy.canNavigate(project, adapterCurrent = true))
        assertTrue(ChatGptWebAccessPolicy.canReadDirectory(project, adapterCurrent = true))
        assertFalse(ChatGptWebAccessPolicy.canChat(project))
    }

    @Test
    fun anAuthenticatedRateLimitDoesNotRemoveDirectoryOrNavigationAccess() {
        val limited = snapshot(true, false, accessReason = "rate_limited")
        assertTrue(ChatGptWebAccessPolicy.canNavigate(limited, true))
        assertTrue(ChatGptWebAccessPolicy.canReadDirectory(limited, true))
        assertFalse(ChatGptWebAccessPolicy.canChat(limited))
    }

    @Test
    fun aStaleDocumentCannotUseCachedAuthenticationForDirectoryOrNavigation() {
        val previous = snapshot(true, true)
        assertFalse(ChatGptWebAccessPolicy.canNavigate(previous, false))
        assertFalse(ChatGptWebAccessPolicy.canReadDirectory(previous, false))
        assertFalse(ChatGptWebAccessPolicy.canNavigate(null, true))
        assertTrue(ChatGptWebAccessPolicy.canReadDirectory(null, true))
        assertFalse(ChatGptWebAccessPolicy.canReadDirectory(null, false))
    }

    @Test
    fun loginEvidenceAndNonChatGptOriginsStillBlockDocumentOperations() {
        val previous = snapshot(true, false)
        val unavailable = listOf(
            previous.copy(loginRequired = true),
            previous.copy(accessReason = "login_required"),
            previous.copy(pageKind = "auth"),
            previous.copy(url = "https://chatgpt.com/auth/login"),
            previous.copy(url = "https://chatgpt.com.evil.example/c/target"),
            previous.copy(url = "http://chatgpt.com/"),
            previous.copy(url = "https://chatgpt.com:8443/"),
        )
        unavailable.forEach { value ->
            assertFalse(ChatGptWebAccessPolicy.canNavigate(value, true))
            assertFalse(ChatGptWebAccessPolicy.canReadDirectory(value, true))
        }
    }

    @Test
    fun anUnknownUiIdentityCanEnterTheReaderWithoutPretendingToBeAuthenticated() {
        val guest = snapshot(false, false)
        assertTrue(ChatGptWebAccessPolicy.canNavigate(guest, true))
        assertTrue(ChatGptWebAccessPolicy.canReadDirectory(guest, true))
        assertFalse(guest.authenticated)
        assertTrue(ChatGptWebAccessPolicy.canReadDirectory(guest.copy(composerReady = true), true))
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
