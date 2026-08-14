package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Test

class GoogleWebNewConversationPolicyTest {
    @Test
    fun ignoresTheOldSnapshotThatCanArriveAfterStartingANewConversation() {
        val previous = snapshot("old", "https://www.google.com/search?q=old&udm=50")

        assertEquals(
            GoogleWebNewConversationTransition.IGNORE_STALE,
            GoogleWebNewConversationPolicy.transition(true, previous, previous),
        )
    }

    @Test
    fun anEmptyPageCompletesTheNewConversationBoundary() {
        assertEquals(
            GoogleWebNewConversationTransition.START_NEW,
            GoogleWebNewConversationPolicy.transition(
                true,
                snapshot("old", "https://www.google.com/search?q=old&udm=50"),
                snapshot("", "https://www.google.com/aimode"),
            ),
        )
    }

    @Test
    fun aFirstMessageCanCompleteTheBoundaryWithoutAnObservedBlankPage() {
        assertEquals(
            GoogleWebNewConversationTransition.START_NEW,
            GoogleWebNewConversationPolicy.transition(
                true,
                snapshot("old", "https://www.google.com/search?q=old&udm=50"),
                snapshot("new", "https://www.google.com/search?q=new&udm=50"),
            ),
        )
    }

    private fun snapshot(query: String, url: String) = ChatGptWebSnapshot(
        title = "Google AI",
        url = url,
        draft = "",
        messages = query.takeIf(String::isNotBlank)?.let {
            listOf(ChatGptWebMessage("user", "user", it, "completed", emptyList()))
        }.orEmpty(),
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
