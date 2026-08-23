package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptRealtimeVoiceRecoveryGateTest {
    @Test
    fun reloadsOnlyWhenTheArmedExitDidNotReceiveAFreshConversationSnapshot() {
        val gate = ChatGptRealtimeVoiceRecoveryGate()
        val token = gate.arm(snapshotRevision = 8L, reloadAllowed = true)

        assertTrue(gate.shouldReload(token, conversationRecoveredSince = false))
        assertFalse(gate.shouldReload(token, conversationRecoveredSince = true))
    }

    @Test
    fun aNewVoiceCycleInvalidatesAnOlderExitCallback() {
        val gate = ChatGptRealtimeVoiceRecoveryGate()
        val oldExit = gate.arm(snapshotRevision = 8L, reloadAllowed = true)

        gate.invalidate()

        assertFalse(gate.shouldReload(oldExit, conversationRecoveredSince = false))
    }

    @Test
    fun onlyTheLatestExitCanRequestRecovery() {
        val gate = ChatGptRealtimeVoiceRecoveryGate()
        val oldExit = gate.arm(snapshotRevision = 8L, reloadAllowed = true)
        val latestExit = gate.arm(snapshotRevision = 9L, reloadAllowed = true)

        assertFalse(gate.shouldReload(oldExit, conversationRecoveredSince = false))
        assertTrue(gate.shouldReload(latestExit, conversationRecoveredSince = false))
    }

    @Test
    fun gracefulExitNeverReloadsAnAlreadySettledConversation() {
        val gate = ChatGptRealtimeVoiceRecoveryGate()
        val token = gate.arm(snapshotRevision = 8L, reloadAllowed = false)

        assertTrue(gate.isCurrent(token))
        assertFalse(gate.shouldReload(token, conversationRecoveredSince = false))
    }

    @Test
    fun conversationRecoveryRequiresAFreshReadySnapshot() {
        val recovery = ChatGptRealtimeVoiceConversationRecovery(snapshot(composerReady = true))
        val baseline = recovery.revision()

        assertFalse(recovery.recoveredSince(baseline))
        recovery.accept(snapshot(composerReady = false))
        assertFalse(recovery.recoveredSince(baseline))
        recovery.accept(snapshot(composerReady = true))

        assertEquals(baseline + 2, recovery.revision())
        assertTrue(recovery.recoveredSince(baseline))
    }

    private fun snapshot(composerReady: Boolean) = ChatGptWebSnapshot(
        title = "会话",
        url = "https://chatgpt.com/c/test",
        draft = "",
        messages = emptyList(),
        authenticated = true,
        composerReady = composerReady,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "conversation",
    )
}
