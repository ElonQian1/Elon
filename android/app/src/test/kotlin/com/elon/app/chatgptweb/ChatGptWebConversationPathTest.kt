package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebConversationPathTest {
    @Test
    fun acceptsOnlyCanonicalChatGptConversationPaths() {
        assertEquals("/c/demo_123", ChatGptWebConversationPath.normalize(" /c/demo_123 "))
        assertEquals(
            "/c/demo-123",
            ChatGptWebConversationPath.fromUrl("https://chatgpt.com/c/demo-123?temporary=true"),
        )
        assertEquals(
            "/g/g-p-demo/c/project-chat",
            ChatGptWebConversationPath.normalize("/g/g-p-demo/c/project-chat"),
        )
        assertNull(ChatGptWebConversationPath.normalize("/g/demo"))
        assertNull(ChatGptWebConversationPath.normalize("https://chatgpt.com/c/demo"))
        assertNull(ChatGptWebConversationPath.normalize("/c/../auth/login"))
        assertNull(ChatGptWebConversationPath.fromUrl("https://example.com/c/demo"))
    }
}
