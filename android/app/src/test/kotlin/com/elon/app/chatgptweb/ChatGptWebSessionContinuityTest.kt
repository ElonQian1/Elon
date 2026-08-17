package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSessionContinuityTest {
    @Test
    fun anonymousComposerClearsPreviouslyObservedAccountSession() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val decision = continuity.reconcileWithDecision(
            snapshot(pageKind = "conversation").copy(composerReady = true),
        )
        val anonymous = decision.snapshot

        assertFalse(anonymous.authenticated)
        assertTrue(decision.clearConversationHistory)
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
    fun transientLoginEvidencePreservesSessionDuringRefreshGracePeriod() {
        var nowMs = 1_000L
        val continuity = ChatGptWebSessionContinuity(
            nowMs = { nowMs },
            loginEvidenceGraceMs = 2_000L,
        )
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val transient = continuity.reconcileWithDecision(
            snapshot(pageKind = "auth", loginRequired = true, authenticated = true),
        )

        assertTrue(transient.snapshot.authenticated)
        assertFalse(transient.snapshot.loginRequired)
        assertFalse(ChatGptWebAccessPolicy.requiresLogin(transient.snapshot))
        assertFalse(transient.clearConversationHistory)
        assertTrue(transient.recheckAfterMs == 2_000L)

        nowMs += 1_000L
        val recovered = continuity.reconcileWithDecision(
            snapshot(authenticated = true, pageKind = "conversation"),
        )
        assertTrue(recovered.snapshot.authenticated)
        assertNull(recovered.recheckAfterMs)
        assertNull(continuity.confirmPendingLoginEvidence())
    }

    @Test
    fun stableLoginEvidenceClearsSessionAfterGracePeriod() {
        var nowMs = 1_000L
        val continuity = ChatGptWebSessionContinuity(
            nowMs = { nowMs },
            loginEvidenceGraceMs = 2_000L,
        )
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))
        continuity.reconcileWithDecision(snapshot(pageKind = "auth", loginRequired = true))
        nowMs += 2_000L

        val login = checkNotNull(continuity.confirmPendingLoginEvidence())

        assertFalse(login.snapshot.authenticated)
        assertTrue(login.snapshot.loginRequired)
        assertTrue(login.clearConversationHistory)
        assertFalse(continuity.reconcile(snapshot(pageKind = "feature")).authenticated)
    }

    @Test
    fun explicitAuthUrlClearsSessionEvenWhenPageKindIsStale() {
        val continuity = ChatGptWebSessionContinuity()
        continuity.reconcile(snapshot(authenticated = true, pageKind = "conversation"))

        val login = continuity.reconcileWithDecision(
            snapshot(
                authenticated = true,
                pageKind = "feature",
                url = "https://chatgpt.com/auth/login",
            ),
        )

        assertFalse(login.snapshot.authenticated)
        assertTrue(login.snapshot.loginRequired)
        assertTrue(login.clearConversationHistory)
    }

    @Test
    fun restoredAuthenticatedCacheGetsRefreshGraceBeforeItCanBeCleared() {
        val continuity = ChatGptWebSessionContinuity(
            initialAuthenticated = true,
            nowMs = { 1_000L },
            loginEvidenceGraceMs = 2_000L,
        )

        val transient = continuity.reconcileWithDecision(
            snapshot(pageKind = "home", loginRequired = true),
        )

        assertTrue(transient.snapshot.authenticated)
        assertFalse(transient.clearConversationHistory)
        assertTrue(transient.recheckAfterMs == 2_000L)
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
