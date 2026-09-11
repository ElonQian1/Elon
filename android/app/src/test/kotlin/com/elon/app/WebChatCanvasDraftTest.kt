package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCanvasComment
import com.elon.app.chatgptweb.ChatGptWebCanvasDocument
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import org.junit.Assert.*
import org.junit.Test

class WebChatCanvasDraftTest {
    private fun document(content: String = "abc DEF ghi", comments: List<ChatGptWebCanvasComment> =
        listOf(ChatGptWebCanvasComment("c", 4, 7, "Keep comment"))) =
        ChatGptWebCanvasDocument("d", "Synthetic", content, "document", 4, comments)
    private fun index(doc: ChatGptWebCanvasDocument, scope: String = "scope", unknown: Boolean = false) =
        ChatGptWebCanvasDocuments("mcp_1", "/c/fixture", "ticket", scope, listOf(doc), unknown)
    private fun draft(doc: ChatGptWebCanvasDocument = document()) = WebChatCanvasDraft("/c/fixture", "scope", doc)

    @Test fun insertBeforeAndInsideCommentAdjustsUtf16Positions() {
        val d = draft()
        d.replace(0, 0, "\uD83D\uDE00")
        assertEquals(6, d.comments.single().start)
        assertEquals(9, d.comments.single().end)
        d.replace(7, 0, "中")
        assertEquals(10, d.comments.single().end)
        assertEquals("Keep comment", d.comments.single().content)
        assertTrue(d.needsRepair.isEmpty())
        assertTrue(d.changed)
    }

    @Test fun deletingEntireAnchorRequiresRepairBeforeSave() {
        val original = document()
        val d = draft(original)
        d.replace(4, 3, "")
        assertEquals(setOf("c"), d.needsRepair)
        assertNull(d.saveRequest(index(original)))
        assertFalse(d.reanchor("c", 1, 1))
        assertFalse(d.reanchor("foreign", 0, 2))
        assertTrue(d.reanchor("c", 0, 3))
        assertNotNull(d.saveRequest(index(original)))
        assertEquals(1, d.comments.size)
    }

    @Test fun splitSurrogateCannotBecomeACommentBoundary() {
        val d = draft(document("\uD83D\uDE00 abc", listOf(ChatGptWebCanvasComment("c", 3, 6, "Comment"))))
        assertFalse(d.reanchor("c", 1, 4))
        assertTrue(d.reanchor("c", 0, 2))
    }

    @Test fun refreshedVersionNeverSilentlyOverwritesTheDraftBase() {
        val original = document()
        val d = draft(original)
        d.replace(0, 3, "mine")
        val text = d.content
        val remote = original.copy(content = "remote content", documentVersion = 5)
        assertNull(d.saveRequest(index(remote)))
        assertEquals(original, d.base)
        assertEquals(text, d.content)
        d.rebase(remote)
        assertEquals(text, d.content)
        assertEquals(remote, d.base)
        assertNull(d.saveRequest(index(remote)))
        assertEquals(setOf("c"), d.needsRepair)
        assertTrue(d.reanchor("c", 0, 4))
        assertNotNull(d.saveRequest(index(remote)))
    }

    @Test fun sameVersionExplicitAcknowledgementKeepsEditedCommentLocations() {
        val original = document()
        val d = draft(original)
        d.replace(0, 0, "Prefix ")
        val comments = d.comments
        d.rebase(original)
        assertEquals(comments, d.comments)
        assertTrue(d.needsRepair.isEmpty())
        assertNotNull(d.saveRequest(index(original)))
    }

    @Test fun identityChangesAndUncertainWritesCannotBeSaved() {
        val original = document()
        val d = draft(original)
        d.replace(0, 0, "edited ")
        assertNull(d.saveRequest(index(original, scope = "other")))
        assertNull(d.saveRequest(index(original, unknown = true)))
        assertNull(d.saveRequest(index(original).copy(path = "/c/other")))
        assertNotNull(d.saveRequest(index(original)))
        assertTrue(d.changed)
        d.adopt(original)
        assertFalse(d.changed)
        assertEquals(original.content, d.content)
    }

    @Test fun replacementAcrossBothEndsAndPartialDeletionStayInBounds() {
        val d = draft()
        d.replace(2, 7, "X")
        assertEquals(2, d.comments.single().start)
        assertEquals(3, d.comments.single().end)
        assertTrue(d.needsRepair.isEmpty())
        val other = draft()
        other.replace(3, 2, "")
        assertEquals(3, other.comments.single().start)
        assertEquals(5, other.comments.single().end)
    }
}
