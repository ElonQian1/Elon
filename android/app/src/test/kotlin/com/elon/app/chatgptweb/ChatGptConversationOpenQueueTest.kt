package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptConversationOpenQueueTest {
    private val queue = ChatGptConversationOpenQueue()

    @Test
    fun latestSelectionWinsWhileTheOriginalConversationRemainsTheFailureFallback() {
        val original = snapshot("original", "/c/original")
        val first = queue.enqueue("/c/first", original)
        val second = queue.enqueue("/c/second", snapshot("preview", "/c/first"))

        assertEquals("/c/first", first.path)
        assertEquals("/c/second", second.path)
        assertEquals(original, second.previousSnapshot)
        assertTrue(queue.hasPending())
        assertEquals(second, queue.take())
        assertFalse(queue.hasPending())
        assertNull(queue.take())
    }

    @Test
    fun clearDropsDeferredNavigation() {
        queue.enqueue("/c/target", null)

        queue.clear()

        assertFalse(queue.hasPending())
    }

    @Test
    fun absentOriginalSnapshotRemainsAbsentWhenTheSelectionChanges() {
        queue.enqueue("/c/first", null)

        val replacement = queue.enqueue("/c/second", snapshot("preview", "/c/first"))

        assertNull(replacement.previousSnapshot)
    }

    private fun snapshot(content: String, path: String) = ChatGptWebSnapshot(
        title = "conversation",
        url = "https://chatgpt.com$path",
        draft = "",
        messages = listOf(ChatGptWebMessage("id", "user", content, "completed", emptyList())),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "auto",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
    )
}
