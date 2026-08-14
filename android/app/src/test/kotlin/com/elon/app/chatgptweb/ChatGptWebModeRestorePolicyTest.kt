package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebModeRestorePolicyTest {
    @Test
    fun keepsExplicitOfficialSelectionInsteadOfRestoringStaleNativeMode() {
        val decision = ChatGptWebModeRestorePolicy.decide(
            pending = ChatGptWebModeController.Mode.NATIVE,
            current = ChatGptWebModeController.Mode.WEB,
            nativeModeEnabled = true,
        )

        assertEquals(ChatGptWebModeController.Mode.WEB, decision.target)
        assertTrue(decision.consumePending)
    }

    @Test
    fun restoresPendingModeWhileControllerIsStillAtBootstrapMode() {
        val decision = ChatGptWebModeRestorePolicy.decide(
            pending = ChatGptWebModeController.Mode.WEB,
            current = ChatGptWebModeController.Mode.QUICK,
            nativeModeEnabled = true,
        )

        assertEquals(ChatGptWebModeController.Mode.WEB, decision.target)
        assertTrue(decision.consumePending)
    }

    @Test
    fun waitsForNativeModeToBecomeAvailableBeforeRestoringIt() {
        val decision = ChatGptWebModeRestorePolicy.decide(
            pending = ChatGptWebModeController.Mode.NATIVE,
            current = ChatGptWebModeController.Mode.QUICK,
            nativeModeEnabled = false,
        )

        assertEquals(null, decision.target)
        assertFalse(decision.consumePending)
    }

    @Test
    fun doesNothingWhenThereIsNoPendingMode() {
        val decision = ChatGptWebModeRestorePolicy.decide(
            pending = null,
            current = ChatGptWebModeController.Mode.QUICK,
            nativeModeEnabled = true,
        )

        assertEquals(null, decision.target)
        assertFalse(decision.consumePending)
    }
}
