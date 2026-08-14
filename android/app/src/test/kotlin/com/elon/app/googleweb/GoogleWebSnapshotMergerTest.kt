package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Test

class GoogleWebSnapshotMergerTest {
    @Test
    fun appendsAFollowUpAndKeepsThePreviousTurn() {
        val first = snapshot("first", "answer one")
        val second = snapshot("second", "answer two")

        val merged = GoogleWebSnapshotMerger.merge(first, second, sameConversation = true)

        assertEquals(listOf("first", "answer one", "second", "answer two"),
            merged.messages.map { it.content })
        assertEquals(listOf(
            "google-message-0-user",
            "google-message-1-assistant",
            "google-message-2-user",
            "google-message-3-assistant",
        ), merged.messages.map { it.id })
    }

    @Test
    fun replacesTheStreamingAnswerForTheCurrentTurn() {
        val streaming = snapshot("question", "partial")
        val completed = snapshot("question", "complete")

        val merged = GoogleWebSnapshotMerger.merge(streaming, completed, sameConversation = true)

        assertEquals(listOf("question", "complete"), merged.messages.map { it.content })
    }

    @Test
    fun switchingConversationDoesNotLeakPreviousMessages() {
        val merged = GoogleWebSnapshotMerger.merge(
            snapshot("private old", "old answer"),
            snapshot("new", "new answer"),
            sameConversation = false,
        )

        assertEquals(listOf("new", "new answer"), merged.messages.map { it.content })
    }

    @Test
    fun anEmptyRefreshKeepsCachedTurnsOnlyForTheSameConversation() {
        val previous = snapshot("question", "answer")
        val empty = snapshot()

        assertEquals(2, GoogleWebSnapshotMerger.merge(previous, empty, true).messages.size)
        assertEquals(0, GoogleWebSnapshotMerger.merge(previous, empty, false).messages.size)
    }

    @Test
    fun boundedHistoryKeepsGlobalMessageIdsAndOffsets() {
        val values = (0 until 34).map { "message-$it" }.toTypedArray()

        val bounded = GoogleWebSnapshotMerger.merge(null, snapshot(*values), false)

        assertEquals(2, bounded.messageWindowStart)
        assertEquals(34, bounded.observedMessageCount)
        assertEquals("google-message-2-user", bounded.messages.first().id)
        assertEquals("google-message-33-assistant", bounded.messages.last().id)
    }

    private fun snapshot(vararg values: String) = ChatGptWebSnapshot(
        title = "Google AI",
        url = "https://www.google.com/search?udm=50&q=test",
        draft = "",
        messages = values.mapIndexed { index, value ->
            val role = if (index % 2 == 0) "user" else "assistant"
            ChatGptWebMessage("current-$role", role, value, "completed", emptyList())
        },
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "Google AI 模式",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "conversation",
    )
}
