package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptConversationHistoryCodecTest {
    @Test
    fun roundTripsOnlySafeBoundedConversationMetadata() {
        val encoded = ChatGptConversationHistoryCodec.encode(
            ChatGptConversationHistoryCache(
                conversations = listOf(
                    ChatGptWebConversation("one", "第一场会话", "/c/one", active = true),
                    ChatGptWebConversation("two", "第二场会话", "/c/two", active = false),
                ),
                savedAtMs = 1234L,
            ),
        )

        val decoded = ChatGptConversationHistoryCodec.decode(encoded)!!

        assertEquals(1234L, decoded.savedAtMs)
        assertEquals(listOf("/c/one", "/c/two"), decoded.conversations.map { it.path })
        assertFalse(decoded.conversations.any { it.active })
    }

    @Test
    fun rejectsUnknownSchemaAndUnsafeOrEmptyIndexes() {
        assertNull(ChatGptConversationHistoryCodec.decode("{}"))
        assertNull(ChatGptConversationHistoryCodec.decode(
            """{"schema":"elon.chatgpt_web.conversation_index.v1","saved_at_ms":1,"conversations":[{"id":"bad","title":"越界","path":"https://example.com/c/bad"}]}""",
        ))
    }
}
