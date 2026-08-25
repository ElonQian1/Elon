package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebSnapshotPresentationTest {
    @Test
    fun openingAVisitedConversationShowsItsCachedBodyImmediately() {
        val cached = previous()

        val opening = GoogleWebSnapshotPresentation.opening(
            cached = cached,
            previous = null,
            url = cached.url,
        )

        assertEquals(cached.messages, opening.messages)
        assertTrue(opening.composerReady)
    }

    @Test
    fun openingAnotherConversationNeverShowsAMismatchedCachedBody() {
        val opening = GoogleWebSnapshotPresentation.opening(
            cached = previous(),
            previous = previous(),
            url = "https://www.google.com/search?q=another&udm=50",
        )

        assertTrue(opening.messages.isEmpty())
        assertFalse(opening.composerReady)
    }

    @Test
    fun loadingAnotherConversationNeverDisplaysOrAuthorizesThePreviousOne() {
        val loading = GoogleWebSnapshotPresentation.loading(previous(), "https://www.google.com/new")

        assertTrue(loading.messages.isEmpty())
        assertFalse(loading.authenticated)
        assertFalse(loading.composerReady)
        assertFalse(loading.streaming)
        assertTrue(loading.capabilities.supported.isEmpty())
        assertEquals("https://www.google.com/new", loading.url)
        assertEquals("Google AI 模式", loading.currentModel)
    }

    private fun previous() = ChatGptWebSnapshot(
        title = "private",
        url = "https://www.google.com/search?q=private&udm=50",
        draft = "private draft",
        messages = listOf(ChatGptWebMessage(
            "message",
            "user",
            "private message",
            "completed",
            emptyList(),
        )),
        authenticated = true,
        composerReady = true,
        streaming = true,
        currentModel = "Google AI 模式",
        attachments = emptyList(),
        dictationActive = true,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
    )
}
