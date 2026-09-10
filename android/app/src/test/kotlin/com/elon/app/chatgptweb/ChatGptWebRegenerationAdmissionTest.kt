package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebRegenerationAdmissionTest {
    @Test
    fun nativeAdmissionDoesNotWaitForDomOrComposerReadiness() {
        assertNull(ChatGptWebRegenerationAdmission.rejection(snapshot()))
    }

    @Test
    fun missingIncompleteAndActiveRepliesRemainIneligible() {
        val unavailable = listOf(
            null, snapshot().copy(messages = emptyList()),
            snapshot(id = ""), snapshot(role = "user"), snapshot(state = "streaming"),
            snapshot(state = "interrupted"), snapshot(state = "error"), snapshot(state = "unknown"),
        )
        unavailable.forEach {
            assertEquals("regenerate_unavailable", ChatGptWebRegenerationAdmission.rejection(it))
        }
        assertEquals(
            "generation_in_progress",
            ChatGptWebRegenerationAdmission.rejection(snapshot().copy(streaming = true)),
        )
    }

    private fun snapshot(id: String = "a1", role: String = "assistant", state: String = "completed") =
        ChatGptWebSnapshot(
            title = "", url = "https://chatgpt.com/c/synthetic", draft = "",
            messages = listOf(ChatGptWebMessage(id, role, "synthetic reply", state, emptyList())),
            authenticated = true, composerReady = false, streaming = false, currentModel = "",
            attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
        )
}
