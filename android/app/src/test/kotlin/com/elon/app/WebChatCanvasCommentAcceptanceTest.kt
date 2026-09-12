package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCanvasComment
import com.elon.app.chatgptweb.ChatGptWebCanvasDocument
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import org.junit.Assert.*
import org.junit.Test

class WebChatCanvasCommentAcceptanceTest {
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val token = "cd_${"a".repeat(32)}_1"
    private val document = ChatGptWebCanvasDocument("synthetic", "Synthetic", "A\uD83D\uDE00BC", "document", 4,
        listOf(ChatGptWebCanvasComment("comment", 1, 3, "Synthetic suggestion")))
    private val index = ChatGptWebCanvasDocuments("mcp_1", path, token, token, listOf(document), false)
    private fun draft() = WebChatCanvasDraft(path, token, document)

    @Test fun acceptanceReferencesTheOriginalCommentWithoutCopyingAPrompt() {
        val request = draft().acceptCommentRequest(index, "comment")!!
        assertEquals("accept_comment", request.getString("operation"))
        assertEquals("comment", request.getString("commentId"))
        assertFalse(request.has("prompt")); assertFalse(request.has("start"))
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(request))
    }

    @Test fun staleDirtyAndUnknownDraftsCannotBeAccepted() {
        val draft = draft()
        assertNull(draft.acceptCommentRequest(index.copy(unconfirmedWrite = true), "comment"))
        assertNull(draft.acceptCommentRequest(index.copy(documents = listOf(document.copy(documentVersion = 5))), "comment"))
        assertNull(draft.acceptCommentRequest(index, "missing"))
        draft.replace(0, 1, "Local")
        assertNull(draft.acceptCommentRequest(index, "comment"))
        assertEquals("Local\uD83D\uDE00BC", draft.content)
    }

    @Test fun resumeOnlyCarriesTheVersionBoundDocumentSelection() {
        val request = draft().selection(index.copy(unconfirmedWrite = true), "resume_comment")
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(request))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(request.put("commentId", "other")))
        val first = draft().acceptCommentRequest(index, "comment")!!
        assertNull(ChatGptWebCanvasDocumentProtocol.request(first.put("prompt", "Forged suggestion")))
    }
}
