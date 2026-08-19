package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebTransientComposerReadinessTest {
    @Test
    fun hiddenOfficialToolMenuDoesNotDowngradeAReadyNativeComposer() {
        val previous = snapshot(composerReady = true)
        val menuOpen = snapshot(composerReady = false).copy(
            capabilities = ChatGptWebCapabilities.EMPTY,
        )

        val result = ChatGptWebTransientComposerReadiness.reconcile(
            previous,
            menuOpen,
            composerInteractionActive = true,
        )

        assertTrue(result.composerReady)
        assertTrue(result.capabilities.supports(ChatGptWebCapabilityId.COMPOSER_TOOLS))
    }

    @Test
    fun normalNavigationAndAuthenticationPagesStillCloseTheComposer() {
        val previous = snapshot(composerReady = true)
        val feature = snapshot(composerReady = false).copy(
            url = "https://chatgpt.com/images",
            pageKind = "feature",
        )
        val auth = snapshot(composerReady = false).copy(
            url = "https://chatgpt.com/auth/login",
            pageKind = "auth",
            loginRequired = true,
        )

        assertFalse(ChatGptWebTransientComposerReadiness.reconcile(
            previous,
            feature,
            composerInteractionActive = true,
        ).composerReady)
        assertFalse(ChatGptWebTransientComposerReadiness.reconcile(
            previous,
            auth,
            composerInteractionActive = true,
        ).composerReady)
    }

    private fun snapshot(composerReady: Boolean) = ChatGptWebSnapshot(
        title = "ChatGPT",
        url = "https://chatgpt.com/c/example",
        draft = "",
        messages = emptyList(),
        authenticated = true,
        composerReady = composerReady,
        streaming = false,
        currentModel = "Fast",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.COMPOSER_TOOLS)),
        pageKind = "conversation",
    )
}
