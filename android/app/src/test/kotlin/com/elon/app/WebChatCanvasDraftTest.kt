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

    @Test fun renamePreservesUnsavedTextAndCommentAnchors() {
        val original = document()
        val d = draft(original)
        d.replace(0, 0, "Prefix ")
        val text = d.content
        val comments = d.comments
        val command = requireNotNull(d.renameRequest(index(original), "New title"))
        assertEquals(setOf("operation", "path", "scope", "ticket", "id", "title"), command.keys().asSequence().toSet())
        assertEquals("rename", command.getString("operation"))
        assertEquals("New title", command.getString("title"))
        val renamed = original.copy(title = "New title")
        assertTrue(d.acceptRename(renamed))
        assertEquals(renamed, d.base)
        assertEquals(text, d.content)
        assertEquals(comments, d.comments)
        assertTrue(d.changed)
        assertTrue(d.needsRepair.isEmpty())
        assertNotNull(d.saveRequest(index(renamed)))
    }

    @Test fun titleOnlyChangesDoNotLoseCommentRepairRequirements() {
        val original = document()
        val d = draft(original)
        d.replace(4, 3, "")
        val text = d.content
        val comments = d.comments
        assertNotNull(d.renameRequest(index(original), "Renamed"))
        val renamed = original.copy(title = "Renamed", documentVersion = 5)
        assertTrue(d.acceptRename(renamed))
        assertEquals(text, d.content)
        assertEquals(comments, d.comments)
        assertEquals(setOf("c"), d.needsRepair)
        assertNull(d.saveRequest(index(renamed)))
    }

    @Test fun renameNeverAdoptsForeignOrModifiedSource() {
        val original = document()
        val d = draft(original)
        d.replace(0, 0, "Mine ")
        val text = d.content
        val titleOnly = original.copy(title = "New title")
        for (remote in listOf(titleOnly.copy(id = "foreign"), titleOnly.copy(content = "Foreign body"),
            titleOnly.copy(documentType = "code/python"), titleOnly.copy(documentVersion = 3),
            titleOnly.copy(comments = emptyList()), original)) {
            assertFalse(d.acceptRename(remote))
            assertEquals(original, d.base)
            assertEquals(text, d.content)
        }
        for (remoteIndex in listOf(index(original, scope = "foreign"), index(original, unknown = true),
            index(original).copy(path = "/c/foreign"), index(titleOnly))) {
            assertNull(d.renameRequest(remoteIndex, "Renamed"))
        }
        for (title in listOf("", " ", " padded", "line\nfeed", "x".repeat(513), "\uD800"))
            assertNull(d.renameRequest(index(original), title))
    }

    @Test fun explicitCommentDismissalPreservesUnsavedBodyAndOtherAnchorRepairs() {
        val original = document(comments = listOf(ChatGptWebCanvasComment("c", 4, 7, "Dismiss"),
            ChatGptWebCanvasComment("keep", 0, 3, "Keep")))
        val d = draft(original)
        d.replace(0, 3, "")
        val text = d.content
        val kept = d.comments.single { it.id == "keep" }
        val request = requireNotNull(d.dismissCommentRequest(index(original), "c"))
        assertEquals(setOf("operation", "path", "scope", "ticket", "id", "commentId"), request.keys().asSequence().toSet())
        assertEquals("dismiss_comment", request.getString("operation"))
        assertEquals("c", request.getString("commentId"))
        val result = original.copy(documentVersion = 5, comments = original.comments.filterNot { it.id == "c" })
        assertTrue(d.acceptCommentDismissal(result))
        assertEquals(result, d.base)
        assertEquals(text, d.content)
        assertEquals(listOf(kept), d.comments)
        assertEquals(setOf("keep"), d.needsRepair)
        assertNull(d.saveRequest(index(result)))
        assertTrue(d.reanchor("keep", 0, 2))
        assertNotNull(d.saveRequest(index(result)))
        assertFalse(d.acceptCommentDismissal(result))
    }

    @Test fun dismissalRequiresIntentAndAnExactServerResult() {
        val original = document()
        val d = draft(original)
        d.replace(4, 3, "")
        val removed = original.copy(documentVersion = 5, comments = emptyList())
        assertFalse(d.acceptCommentDismissal(removed))
        for (value in listOf(index(original, unknown = true), index(original, scope = "foreign"),
            index(original).copy(path = "/c/foreign"), index(original.copy(documentVersion = 5))))
            assertNull(d.dismissCommentRequest(value, "c"))
        assertNull(d.dismissCommentRequest(index(original), "foreign"))
        assertNotNull(d.dismissCommentRequest(index(original), "c"))
        for (value in listOf(original, removed.copy(documentVersion = 4), removed.copy(id = "foreign"),
            removed.copy(content = "Other body"), removed.copy(title = "Other title"), removed.copy(documentType = "code/python"))) {
            assertFalse(d.acceptCommentDismissal(value))
            assertEquals(original, d.base)
            assertEquals(setOf("c"), d.needsRepair)
        }
        val text = d.content
        assertTrue(d.acceptCommentDismissal(removed))
        assertEquals(text, d.content)
        assertTrue(d.needsRepair.isEmpty())
        assertNotNull(d.saveRequest(index(removed)))
    }

    @Test fun reviewingOrAdoptingAnotherBaseClearsTheOldDismissalIntent() {
        val original = document()
        val d = draft(original)
        assertNotNull(d.dismissCommentRequest(index(original), "c"))
        d.rebase(original)
        assertFalse(d.acceptCommentDismissal(original.copy(documentVersion = 5, comments = emptyList())))
        assertNotNull(d.dismissCommentRequest(index(original), "c"))
        d.adopt(original)
        assertFalse(d.acceptCommentDismissal(original.copy(documentVersion = 5, comments = emptyList())))
    }
}
