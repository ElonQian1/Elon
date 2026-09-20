package com.elon.app.chatgptweb

import com.elon.app.WebChatProviderId
import org.junit.Assert.*
import org.junit.Test

class GroupWebAiSessionPolicyTest {
    private val provider = WebChatProviderId.CHATGPT_WEB
    private val document = GroupWebAiSessionPolicy.startUrl(provider)
    private fun snapshot() = ChatGptWebSnapshot(
        title = "", url = "https://chatgpt.com/", draft = "", messages = emptyList(),
        authenticated = true, composerReady = true, streaming = false, currentModel = "",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
    )

    @Test fun canonicalSnapshotUsesLiveTemporaryDocumentWithoutExportingQuery() {
        val value = snapshot()
        assertFalse(GroupWebAiSessionPolicy.ready(value, provider))
        assertTrue(GroupWebAiSessionPolicy.ready(value, provider, document))
        assertEquals("https://chatgpt.com/", value.url)
        assertTrue(GroupWebAiSessionPolicy.ready(value.copy(composerReady = false, privateSendReady = true), provider, document))
    }

    @Test fun temporaryFlagCannotBeBorrowedFromAnotherOrUnsafeDocument() {
        listOf("", "https://chatgpt.com/", "https://chatgpt.com/c/old?temporary-chat=true",
            "https://chatgpt.com/?temporary-chat=false", "https://chatgpt.com/?temporary-chat=true&gizmo_id=other",
            "https://chatgpt.com/?temporary-chat=true#other", "https://chatgpt.com:444/?temporary-chat=true",
            "https://user@chatgpt.com/?temporary-chat=true", "https://chatgpt.com.evil.test/?temporary-chat=true"
        ).forEach { assertFalse(it, GroupWebAiSessionPolicy.ready(snapshot(), provider, it)) }
        listOf("https://chatgpt.com/c/old", "https://other.test/", "http://chatgpt.com/",
            "https://user@chatgpt.com/").forEach {
            assertFalse(it, GroupWebAiSessionPolicy.ready(snapshot().copy(url = it), provider, document))
        }
    }

    @Test fun liveTemporaryRouteDoesNotBypassContentAndLoginGuards() {
        listOf(snapshot().copy(draft = "unsent"), snapshot().copy(streaming = true),
            snapshot().copy(loginRequired = true), snapshot().copy(composerReady = false),
            snapshot().copy(messages = listOf(ChatGptWebMessage("old", "user", "old", "completed", emptyList())))
        ).forEach { assertFalse(GroupWebAiSessionPolicy.ready(it, provider, document)) }
    }
}
