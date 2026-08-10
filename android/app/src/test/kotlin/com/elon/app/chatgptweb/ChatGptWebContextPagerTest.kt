package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebContextPagerTest {
    @Test
    fun cursorContinuesTheSameSnapshotWithoutRepeatingMessages() {
        val snapshot = snapshot("/c/one", "first", "second", "third")
        val first = success(ChatGptWebContextPager.page(snapshot, "", 0, 2, 40))
        val second = success(ChatGptWebContextPager.page(
            snapshot,
            requireNotNull(first.nextCursor),
            requestedOffset = 0,
            requestedLimit = 2,
            maxLimit = 40,
        ))

        assertEquals(listOf("first", "second"), first.messages.map(ChatGptWebMessage::content))
        assertEquals(listOf("third"), second.messages.map(ChatGptWebMessage::content))
        assertEquals(2, second.offset)
        assertFalse(second.hasMore)
        assertNull(second.nextCursor)
    }

    @Test
    fun cursorFailsClosedWhenConversationContentChanges() {
        val firstSnapshot = snapshot("/c/one", "first", "streaming")
        val first = success(ChatGptWebContextPager.page(firstSnapshot, "", 0, 1, 40))
        val changedSnapshot = snapshot("/c/one", "first", "completed")
        val result = ChatGptWebContextPager.page(
            changedSnapshot,
            requireNotNull(first.nextCursor),
            requestedOffset = 0,
            requestedLimit = 1,
            maxLimit = 40,
        )

        assertTrue(result is ChatGptWebContextPager.Result.Failure)
        result as ChatGptWebContextPager.Result.Failure
        assertEquals("stale_context_cursor", result.code)
        assertNotEquals(first.revision, result.currentRevision)
    }

    @Test
    fun malformedCursorIsRejectedInsteadOfFallingBackToOffsetZero() {
        val result = ChatGptWebContextPager.page(
            snapshot("/c/one", "first"),
            cursor = "ctx1.invalid.0",
            requestedOffset = 0,
            requestedLimit = 20,
            maxLimit = 40,
        )

        assertTrue(result is ChatGptWebContextPager.Result.Failure)
        assertEquals(
            "invalid_context_cursor",
            (result as ChatGptWebContextPager.Result.Failure).code,
        )
    }

    @Test
    fun revisionChangesWhenTheConversationIdentityChanges() {
        val first = ChatGptWebContextPager.revision(snapshot("/c/one", "same"))
        val second = ChatGptWebContextPager.revision(snapshot("/c/two", "same"))

        assertNotEquals(first, second)
    }

    @Test
    fun longConversationUsesGlobalOffsetsAndRejectsUnavailableHistory() {
        val snapshot = snapshot(
            "/c/long",
            "eighty",
            "eighty-one",
            messageWindowStart = 80,
            observedMessageCount = 82,
        )

        val page = success(ChatGptWebContextPager.page(snapshot, "", 80, 1, 40))
        assertEquals(80, page.offset)
        assertEquals(81, page.nextOffset)
        assertTrue(page.hasMore)
        assertTrue(page.hasMoreBefore)

        val unavailable = ChatGptWebContextPager.page(snapshot, "", 0, 1, 40)
        assertEquals(
            "context_history_unavailable",
            (unavailable as ChatGptWebContextPager.Result.Failure).code,
        )
        assertEquals(80, unavailable.messageWindowStart)
        assertEquals(82, unavailable.messageWindowEnd)
    }

    private fun success(result: ChatGptWebContextPager.Result): ChatGptWebContextPager.Page {
        assertTrue(result is ChatGptWebContextPager.Result.Success)
        return (result as ChatGptWebContextPager.Result.Success).page
    }

    private fun snapshot(
        path: String,
        vararg content: String,
        messageWindowStart: Int = 0,
        observedMessageCount: Int = messageWindowStart + content.size,
    ): ChatGptWebSnapshot = ChatGptWebSnapshot(
        title = "测试会话",
        url = "https://chatgpt.com$path",
        draft = "",
        messages = content.mapIndexed { index, value ->
            ChatGptWebMessage(
                id = "message-$index",
                role = if (index % 2 == 0) "user" else "assistant",
                content = value,
                state = "completed",
                parts = emptyList(),
            )
        },
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        messageWindowStart = messageWindowStart,
        observedMessageCount = observedMessageCount,
    )
}
