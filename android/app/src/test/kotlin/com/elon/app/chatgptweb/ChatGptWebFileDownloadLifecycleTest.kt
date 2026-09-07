package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebFileDownloadLifecycleTest {
    @Test fun downloadCommandWaitsForStorageWhileOtherCommandTimeoutsStayUnchanged() {
        var now = 1_000L
        val state = ChatGptWebObservedState(nowMs = { now })
        val download = state.beginCommand("download_conversation_file")
        val ordinary = state.beginCommand("list_conversations")
        now += 21_000
        var commands = state.snapshot().commandRequests
        assertEquals(ChatGptWebObservedState.CommandRequest.PENDING, commands.first { it.id == download.id }.status)
        assertEquals(ChatGptWebObservedState.CommandRequest.TIMED_OUT, commands.first { it.id == ordinary.id }.status)
        state.accept(ChatGptWebEvent.CommandResult("download_conversation_file", true, "download_saved", download.id))
        commands = state.snapshot().commandRequests
        assertEquals(ChatGptWebObservedState.CommandRequest.SUCCEEDED, commands.first { it.id == download.id }.status)
        val expired = state.beginCommand("download_conversation_file")
        now += ChatGptWebFileByteTransfer.COMMAND_TIMEOUT_MS
        assertEquals(ChatGptWebObservedState.CommandRequest.TIMED_OUT,
            state.snapshot().commandRequests.first { it.id == expired.id }.status)
    }
}
