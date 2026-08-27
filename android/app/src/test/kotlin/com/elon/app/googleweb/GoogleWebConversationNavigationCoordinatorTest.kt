package com.elon.app.googleweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebConversationNavigationCoordinatorTest {
    private val coordinator = GoogleWebConversationNavigationCoordinator { it }

    @Test
    fun openingHistoryIgnoresThePreviousPageSnapshot() {
        coordinator.beginOpen("/google-ai-mode/conversation/target", "https://google.test/target")

        assertFalse(coordinator.shouldAccept("https://google.test/old"))
        assertTrue(coordinator.hasPending())
        assertTrue(coordinator.shouldAccept("https://google.test/target"))
        assertFalse(coordinator.hasPending())
    }

    @Test
    fun aSecondSelectionReplacesTheFirstPendingTarget() {
        coordinator.beginOpen("/google-ai-mode/conversation/first", "https://google.test/first")
        coordinator.beginOpen("/google-ai-mode/conversation/second", "https://google.test/second")

        assertFalse(coordinator.shouldAccept("https://google.test/first"))
        assertTrue(coordinator.shouldAccept("https://google.test/second"))
    }

    @Test
    fun cancellingRestoresNormalSnapshotAcceptance() {
        coordinator.beginOpen("/google-ai-mode/conversation/target", "https://google.test/target")
        coordinator.cancel()

        assertTrue(coordinator.shouldAccept("https://google.test/other"))
    }

    @Test
    fun defaultPolicyDoesNotAcceptATransientPromptUrlForAHistoryOpen() {
        val stableUrl = "https://www.google.com/search?q=first&udm=50&csuir=thread-123"
        val defaultCoordinator = GoogleWebConversationNavigationCoordinator()
        defaultCoordinator.beginOpen("/google-ai-mode/conversation/target", stableUrl)

        assertFalse(defaultCoordinator.shouldAccept(
            "https://www.google.com/search?q=first&udm=50",
        ))
        assertTrue(defaultCoordinator.shouldAccept(stableUrl))
    }
}
