package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatPendingSendStateTest {
    @Test
    fun observedUserMessageAndTransientEmptySnapshotKeepPendingTurnVisible() {
        val state = WebChatPendingSendState()
        state.begin("你好")

        assertTrue(state.observeSubmission(" 你好 "))
        assertFalse(state.observeCompletedTurn(null, assistantObserved = false))
        assertEquals("你好", state.prompt())
        assertEquals(WebChatPendingSendState.Phase.AWAITING_RESPONSE, state.phase())
    }

    @Test
    fun assistantObservationCompletesThePendingTurn() {
        val state = WebChatPendingSendState()
        state.begin("你好")
        state.confirmSubmission()

        assertTrue(state.observeCompletedTurn("你好", assistantObserved = true))
        assertNull(state.prompt())
        assertEquals(WebChatPendingSendState.Phase.IDLE, state.phase())
    }

    @Test
    fun pendingSendRetainsPreviousMessagesAcrossATransientEmptySnapshot() {
        val previous = snapshot(
            messages = listOf(ChatGptWebMessage("u1", "user", "旧问题", "completed", emptyList())),
        )
        val incoming = snapshot(messages = emptyList())

        val presented = WebChatPendingSendSnapshotPresentation.resolve(
            previous = previous,
            incoming = incoming,
            pending = true,
        )

        assertEquals(previous.messages, presented.messages)
        assertEquals(incoming.currentModel, presented.currentModel)
    }

    @Test
    fun explicitEmptyConversationIsNotReplacedWhenNoSendIsPending() {
        val previous = snapshot(
            messages = listOf(ChatGptWebMessage("u1", "user", "旧问题", "completed", emptyList())),
        )
        val incoming = snapshot(messages = emptyList())

        assertEquals(
            incoming,
            WebChatPendingSendSnapshotPresentation.resolve(
                previous = previous,
                incoming = incoming,
                pending = false,
            ),
        )
    }

    private fun snapshot(messages: List<ChatGptWebMessage>) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "GPT-5",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
    )
}
