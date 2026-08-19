package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptNewConversationRecoveryPolicyTest {
    @Test
    fun reloadsAHomeRouteThatDidNotRestoreItsComposer() {
        assertEquals(
            ChatGptNewConversationRecoveryAction.RELOAD_HOME,
            action(webViewAtHome = true),
        )
    }

    @Test
    fun loadsHomeWhenTheOfficialClickDidNotLeaveTheConversation() {
        assertEquals(
            ChatGptNewConversationRecoveryAction.LOAD_HOME,
            action(webViewAtHome = false),
        )
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
        webViewAtHome: Boolean = true,
    ) = ChatGptNewConversationRecoveryPolicy.action(
        navigationActive = navigationActive,
        loading = loading,
        composerReady = composerReady,
        webViewAtHome = webViewAtHome,
    )
}
