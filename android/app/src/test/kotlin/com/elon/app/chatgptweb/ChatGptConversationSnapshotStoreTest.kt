package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptConversationSnapshotStoreTest {
    @Test
    fun createsTheSameSafeFileNameForGlobalAndProjectPathsWithTheSameConversationId() {
        val global = ChatGptConversationSnapshotStore.fileName("/c/conversation_123")
        val project = ChatGptConversationSnapshotStore.fileName(
            "/g/g-p-project/c/conversation_123",
        )

        assertNotNull(global)
        assertEquals(global, project)
        assertEquals(64, global!!.removePrefix("chatgpt-web-conversation-").substringBefore("-v1").length)
    }

    @Test
    fun rejectsNonConversationAndTraversalPaths() {
        assertNull(ChatGptConversationSnapshotStore.fileName("/"))
        assertNull(ChatGptConversationSnapshotStore.fileName("/auth/login"))
        assertNull(ChatGptConversationSnapshotStore.fileName("/c/../../private"))
        assertNull(ChatGptConversationSnapshotStore.fileName("https://chatgpt.com/c/demo"))
    }
}
