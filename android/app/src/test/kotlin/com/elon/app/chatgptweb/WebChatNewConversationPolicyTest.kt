package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatNewConversationPolicyTest {
    @Test
    fun ignoresTheSameCompletedTurnWhileWaitingForANewConversation() {
        val previous = snapshot("旧问题", "conversation-old")

        assertEquals(
            WebChatNewConversationTransition.IGNORE_STALE,
            WebChatNewConversationPolicy.transition(true, previous, previous, ::location),
        )
    }

    @Test
    fun acceptsAnEmptyBoundaryOrADifferentConversation() {
        val previous = snapshot("旧问题", "conversation-old")

        assertEquals(
            WebChatNewConversationTransition.START_NEW,
            WebChatNewConversationPolicy.transition(
                true,
                previous,
                snapshot("", "home"),
                ::location,
            ),
        )
        assertEquals(
            WebChatNewConversationTransition.START_NEW,
            WebChatNewConversationPolicy.transition(
                true,
                previous,
                snapshot("新问题", "conversation-new"),
                ::location,
            ),
        )
    }

    @Test
    fun doesNotFilterNormalSnapshotsWithoutAPendingBoundary() {
        assertEquals(
            WebChatNewConversationTransition.CONTINUE_CURRENT,
            WebChatNewConversationPolicy.transition(
                false,
                snapshot("旧问题", "conversation-old"),
                snapshot("旧问题", "conversation-old"),
                ::location,
            ),
        )
    }

    private fun location(url: String): String = url.substringAfterLast('/')

    private fun snapshot(content: String, location: String) = ChatGptWebSnapshot(
        title = "会话",
        url = "https://example.com/$location",
        draft = "",
        messages = content.takeIf(String::isNotBlank)?.let {
            listOf(ChatGptWebMessage("id", "user", it, "completed", emptyList()))
        }.orEmpty(),
        authenticated = false,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "conversation",
    )
}
