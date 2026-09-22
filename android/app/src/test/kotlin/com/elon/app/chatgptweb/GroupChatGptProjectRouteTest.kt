package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class GroupChatGptProjectRouteTest {
    private val project = "g-p-" + "a".repeat(32)
    private val id = "22222222-2222-4222-8222-222222222222"
    private val url = "https://chatgpt.com/g/$project/c/$id"
    private fun snapshot() = ChatGptWebSnapshot("", url, "", listOf(
        ChatGptWebMessage("old", "user", "prior context", "completed", emptyList())),
        true, true, false, currentModel = "", attachments = emptyList(), dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY)

    @Test fun persistentProjectAcceptsHistoryButNotDifferentSnapshotOrDraft() {
        assertTrue(GroupChatGptProjectRoute.ready(snapshot(), url))
        assertFalse(GroupChatGptProjectRoute.ready(snapshot().copy(url = "https://chatgpt.com/"), url))
        assertFalse(GroupChatGptProjectRoute.ready(snapshot().copy(draft = "unsent"), url))
        assertFalse(GroupChatGptProjectRoute.ready(snapshot().copy(streaming = true), url))
        assertFalse(GroupChatGptProjectRoute.ready(snapshot().copy(authenticated = false), url))
    }
    @Test fun unsafeOrTemporaryRoutesCannotOwnPersistentGroupHistory() {
        for (bad in listOf("$url?temporary-chat=true", "$url#other", url.replace("https:", "http:"),
            url.replace("chatgpt.com", "chatgpt.com.evil.test"), url.replace("chatgpt.com", "user@chatgpt.com"))) {
            assertNull(GroupChatGptProjectRoute.conversationId(bad))
            assertFalse(GroupChatGptProjectRoute.ready(snapshot().copy(url = bad), bad))
        }
        assertEquals(id, GroupChatGptProjectRoute.conversationId(url))
        assertEquals(id, GroupChatGptProjectRoute.conversationId("https://chatgpt.com/c/$id"))
    }
}
