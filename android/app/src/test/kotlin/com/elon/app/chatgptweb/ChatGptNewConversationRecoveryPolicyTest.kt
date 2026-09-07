package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptNewConversationRecoveryPolicyTest {
    @Test
    fun asksForEvidenceWhileTheCurrentPageCommandIsUnsettled() {
        assertEquals(
            ChatGptNewConversationRecoveryAction.REQUEST_SNAPSHOT,
            action(),
        )
    }

    @Test
    fun recoveryCannotReplaceThePageOrDiscardAGuestConversation() {
        assertEquals(
            setOf("NONE", "REQUEST_SNAPSHOT"),
            ChatGptNewConversationRecoveryAction.values().map { it.name }.toSet(),
        )
    }

    @Test
    fun onlyAnActiveLoadingNavigationWithoutAComposerNeedsAProbe() {
        for (active in listOf(false, true)) {
            for (loading in listOf(false, true)) {
                for (ready in listOf(false, true)) {
                    val expected = if (active && loading && !ready) {
                        ChatGptNewConversationRecoveryAction.REQUEST_SNAPSHOT
                    } else ChatGptNewConversationRecoveryAction.NONE
                    assertEquals(expected, action(active, loading, ready))
                }
            }
        }
    }

    @Test
    fun doesNothingAfterNavigationCompletesOrOutsideNavigation() {
        assertEquals(
            ChatGptNewConversationRecoveryAction.NONE,
            action(navigationActive = false),
        )
        assertEquals(
            ChatGptNewConversationRecoveryAction.NONE,
            action(composerReady = true),
        )
        assertEquals(
            ChatGptNewConversationRecoveryAction.NONE,
            action(loading = false),
        )
    }

    private fun action(
        navigationActive: Boolean = true,
        loading: Boolean = true,
        composerReady: Boolean = false,
    ) = ChatGptNewConversationRecoveryPolicy.action(
        navigationActive = navigationActive,
        loading = loading,
        composerReady = composerReady,
    )
}
