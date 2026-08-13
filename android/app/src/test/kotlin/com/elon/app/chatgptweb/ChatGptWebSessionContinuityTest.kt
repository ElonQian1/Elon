package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSessionContinuityTest {
    @Test
    fun anonymousComposerClearsPreviouslyObservedAccountSession() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val anonymous = continuity.reconcile(
            snapshot(pageKind = "conversation").copy(composerReady = true),
        )

        assertFalse(anonymous.authenticated)
        assertFalse(continuity.reconcile(snapshot(pageKind = "feature")).authenticated)
    }

    @Test
    fun doesNotPromoteFeaturePageWithoutPriorAuthenticationEvidence() {
        val continuity = ChatGptWebSessionContinuity()

        assertFalse(continuity.reconcile(snapshot(pageKind = "feature")).authenticated)
    }

    @Test
    fun preservesObservedSessionOnComposerlessFeaturePage() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val feature = continuity.reconcile(snapshot(pageKind = "feature"))

        assertTrue(feature.authenticated)
        assertFalse(feature.loginRequired)
    }

    @Test
    fun preservesObservedSessionWhileAuthenticatedOverlayHidesTheComposer() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(
            snapshot(
                authenticated = true,
                pageKind = "home",
                url = "https://chatgpt.com/",
            ),
        )

        val overlay = continuity.reconcile(
            snapshot(pageKind = "home", url = "https://chatgpt.com/"),
        )

        assertTrue(overlay.authenticated)
        assertFalse(overlay.loginRequired)
    }

    @Test
    fun visibleLoginEvidenceClearsSessionContinuity() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val login = continuity.reconcile(
            snapshot(pageKind = "feature", loginRequired = true, authenticated = true),
        )

        assertFalse(login.authenticated)
        assertTrue(login.loginRequired)
        assertFalse(continuity.reconcile(snapshot(pageKind = "feature")).authenticated)
    }

    @Test
    fun explicitAuthUrlClearsSessionEvenWhenPageKindIsStale() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val login = continuity.reconcile(
            snapshot(
                authenticated = true,
                pageKind = "feature",
                url = "https://chatgpt.com/auth/login",
            ),
        )

        assertFalse(login.authenticated)
        assertTrue(login.loginRequired)
    }

    @Test
    fun clearRemovesObservedSession() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        continuity.clear()

        assertFalse(continuity.reconcile(snapshot(pageKind = "feature")).authenticated)
    }

    private fun snapshot(
        authenticated: Boolean = false,
        pageKind: String,
        loginRequired: Boolean = false,
        url: String = "https://chatgpt.com/tasks",
    ) = ChatGptWebSnapshot(
        title = "ChatGPT",
        url = url,
        draft = "",
        messages = emptyList(),
        authenticated = authenticated,
        composerReady = false,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(emptySet()),
        pageKind = pageKind,
        loginRequired = loginRequired,
    )
}
