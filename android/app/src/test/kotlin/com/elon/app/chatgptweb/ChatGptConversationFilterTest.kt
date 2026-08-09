package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptConversationFilterTest {
    private val conversations = listOf(
        ChatGptWebConversation("one", "ELON bridge verification", "/c/one", active = true),
        ChatGptWebConversation("two", "Android WebView login", "/c/two", active = false),
        ChatGptWebConversation("three", "Football API review", "/c/three", active = false),
    )

    @Test
    fun blankQueryPreservesOfficialOrder() {
        assertEquals(conversations, ChatGptConversationFilter.apply(conversations, "  "))
    }

    @Test
    fun titleSearchIsTrimmedAndCaseInsensitive() {
        assertEquals(
            listOf(conversations[1]),
            ChatGptConversationFilter.apply(conversations, " webVIEW "),
        )
    }

    @Test
    fun unmatchedQueryReturnsEmptyList() {
        assertEquals(emptyList<ChatGptWebConversation>(), ChatGptConversationFilter.apply(conversations, "missing"))
    }
}
