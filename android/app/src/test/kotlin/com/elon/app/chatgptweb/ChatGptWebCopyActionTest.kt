package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebCopyActionTest {
    @Test
    fun copiesCompletedAssistantWithoutExportingContent() {
        var copied = ""
        val result = ChatGptWebCopyAction.execute(snapshot()) { text ->
            copied = text
            ChatGptClipboardMetadata(true, 1, setOf("text/plain"))
        }

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("private response", copied)
        val receipt = result.getJSONObject("receipt")
        assertTrue(receipt.getBoolean("copied"))
        assertEquals(1, receipt.getInt("item_count"))
        assertFalse(receipt.getBoolean("content_exported"))
        assertFalse(result.toString().contains("private response"))
    }

    @Test
    fun rejectsUnavailableOrStreamingResponsesWithoutClipboardAccess() {
        var writes = 0
        val copy = { _: String ->
            writes += 1
            ChatGptClipboardMetadata(true, 1, setOf("text/plain"))
        }
        val unavailable = ChatGptWebCopyAction.execute(
            snapshot(capabilities = emptySet()),
            copy,
        )
        val streaming = ChatGptWebCopyAction.execute(snapshot(streaming = true), copy)

        assertEquals("copy_unavailable", unavailable.getString("error"))
        assertEquals("generation_in_progress", streaming.getString("error"))
        assertEquals(0, writes)
    }

    @Test
    fun convertsClipboardFailureIntoPrivacySafeError() {
        val result = ChatGptWebCopyAction.execute(snapshot()) { error("clipboard denied") }

        assertFalse(result.getBoolean("control_ok"))
        assertEquals("clipboard_write_failed", result.getString("error"))
        assertFalse(result.toString().contains("clipboard denied"))
    }

    private fun snapshot(
        streaming: Boolean = false,
        capabilities: Set<String> = setOf(ChatGptWebCapabilityId.MESSAGE_COPY),
    ) = ChatGptWebSnapshot(
        title = "Test",
        url = "https://chatgpt.com/c/test",
        draft = "",
        messages = listOf(
            ChatGptWebMessage(
                id = "assistant-test",
                role = "assistant",
                content = "private response",
                state = "completed",
                parts = emptyList(),
            ),
        ),
        authenticated = true,
        composerReady = true,
        streaming = streaming,
        currentModel = "model",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(capabilities),
    )
}
