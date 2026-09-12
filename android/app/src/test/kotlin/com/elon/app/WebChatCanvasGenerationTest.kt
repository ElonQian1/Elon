package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCanvasDocument
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import org.junit.Assert.*
import org.junit.Test

class WebChatCanvasGenerationTest {
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val token = "cd_${"a".repeat(32)}_1"
    private val doc = ChatGptWebCanvasDocument("synthetic", "Synthetic", "A\uD83D\uDE00BC", "document", 4, emptyList())
    private fun index() = ChatGptWebCanvasDocuments("mcp_1", path, token, token, listOf(doc), false)
    private fun draft() = WebChatCanvasDraft(path, token, doc)

    @Test fun keepsUtf16SelectionAndVersionBoundTicket() {
        val value = draft().generationRequest(index(), "Rewrite synthetic selection", 1, 3)!!
        assertEquals("generate", value.getString("operation"))
        assertEquals(token, value.getString("ticket"))
        assertEquals(1, value.getInt("start")); assertEquals(3, value.getInt("end"))
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(value))
    }

    @Test fun rejectsUnsavedOrChangedOriginalAndUnknownResult() {
        val editing = draft()
        assertNull(editing.generationRequest(index().copy(unconfirmedWrite = true), "Rewrite", 0, 0))
        assertNull(editing.generationRequest(index().copy(documents = listOf(doc.copy(documentVersion = 5))), "Rewrite", 0, 0))
        editing.replace(0, 1, "Draft")
        assertNull(editing.generationRequest(index(), "Rewrite", 0, 0))
        assertEquals("Draft\uD83D\uDE00BC", editing.content)
    }

    @Test fun rejectsBrokenSelectionsAndInvalidPrompts() {
        val editing = draft()
        for ((start, end) in listOf(-1 to 0, 2 to 3, 1 to 2, 4 to 1, 0 to 999))
            assertNull(editing.generationRequest(index(), "Rewrite", start, end))
        for (prompt in listOf("", " ", "x".repeat(4001), "\uD800"))
            assertNull(editing.generationRequest(index(), prompt, 0, 0))
        val value = editing.generationRequest(index(), "Rewrite", 0, 0)!!.put("headers", "forbidden")
        assertNull(ChatGptWebCanvasDocumentProtocol.request(value))
    }
}
