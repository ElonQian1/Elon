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
    fun keepsTheCurrentAnswerWhenAReflowSnapshotOnlyContainsTheQuestion() {
        val answered = snapshot("What is 2 plus 3?", "5")
        val reflowed = snapshot("What is 2 plus 3?")

        val merged = GoogleWebSnapshotMerger.merge(answered, reflowed, sameConversation = true)

        assertEquals(listOf("What is 2 plus 3?", "5"), merged.messages.map { it.content })
    }

    @Test
    fun partialFirstTurnRefreshDoesNotEraseCachedFollowUps() {
        val cached = snapshot("first", "first answer", "second", "second answer")
        val partial = snapshot("first", "first answer")

        val merged = GoogleWebSnapshotMerger.merge(cached, partial, sameConversation = true)

        assertEquals(
            listOf("first", "first answer", "second", "second answer"),
            merged.messages.map { it.content },
        )
    }

    @Test
    fun fullDomRefreshUpdatesAllTurnsWithoutDuplicatingHistory() {
        val cached = snapshot("first", "old first", "second", "old second")
        val refreshed = snapshot("first", "fresh first", "second", "fresh second")

        val merged = GoogleWebSnapshotMerger.merge(cached, refreshed, sameConversation = true)

        assertEquals(
            listOf("first", "fresh first", "second", "fresh second"),
            merged.messages.map { it.content },
        )
    }

    @Test
    fun previousAnswerCannotBeCarriedAcrossANewQuestion() {
        val answered = snapshot("first", "answer one")

        val stale = GoogleWebSnapshotMerger.merge(
            answered,
            snapshot("second", "answer one"),
            sameConversation = true,
        )

        assertEquals(
            listOf("first", "answer one", "second"),
            stale.messages.map { it.content },
        )

        val refreshed = GoogleWebSnapshotMerger.merge(
            stale,
            snapshot("second", "answer two"),
            sameConversation = true,
        )

        assertEquals(
            listOf("first", "answer one", "second", "answer two"),
            refreshed.messages.map { it.content },
        )
    }

    @Test
    fun completedStreamingTurnMayLegitimatelyRepeatAnEarlierAnswer() {
        val waiting = GoogleWebSnapshotMerger.merge(
            snapshot("first", "same answer"),
            snapshot("second", "same answer").copy(streaming = true),
            sameConversation = true,
        )

        val completed = GoogleWebSnapshotMerger.merge(
            waiting,
            snapshot("second", "same answer"),
            sameConversation = true,
        )

        assertEquals(
            listOf("first", "same answer", "second", "same answer"),
            completed.messages.map { it.content },
        )
    }

    @Test
    fun laterQuestionProvesCachedRepeatedAnswerWasCarryOver() {
        val corrupted = snapshot(
            "first", "same answer",
            "second", "same answer",
            "third",
        )

        val sanitized = GoogleWebSnapshotMerger.sanitizeCached(corrupted)

        assertEquals(
            listOf("first", "same answer", "second", "third"),
            sanitized.messages.map { it.content },
        )
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
        val values = (0 until 82).map { "message-$it" }.toTypedArray()

        val bounded = GoogleWebSnapshotMerger.merge(null, snapshot(*values), false)

        assertEquals(2, bounded.messageWindowStart)
        assertEquals(82, bounded.observedMessageCount)
        assertEquals("google-message-2-user", bounded.messages.first().id)
        assertEquals("google-message-81-assistant", bounded.messages.last().id)
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
