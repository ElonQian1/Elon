package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class ChatSelectionIdentityTest {
    @Test fun historyInsertionAndReorderKeepOriginalSelection() {
        val a = message("a", "first")
        val b = message("b", "second")
        val identity = ChatSelectionIdentity()
        identity.select(b)
        assertEquals(listOf("b"), identity.messagesInOrder(listOf(message("older", "old"), a, b)).map { it.id })
        identity.select(a)
        assertEquals(listOf("b", "a"), identity.messagesInOrder(listOf(b, a)).map { it.id })
    }

    @Test fun streamingRetainsSelectedContentAndRevision() {
        val row = message("answer", "original").apply { revision = 3 }
        val identity = ChatSelectionIdentity()
        identity.select(row)
        val before = identity.metadataInOrder(listOf(row)).single()
        row.content += " new tokens"
        row.revision = 4
        val frozen = identity.messagesInOrder(listOf(row)).single()
        assertEquals("original", frozen.content)
        assertEquals(3L, frozen.revision)
        assertEquals(before, identity.metadataInOrder(listOf(row)).single())
        frozen.content = "mutated by caller"
        assertEquals("original", identity.messagesInOrder(listOf(row)).single().content)
    }

    @Test fun remappedServerMessageKeepsSnapshotNotReplacementBody() {
        val original = message("answer", "selected")
        val identity = ChatSelectionIdentity()
        identity.select(original)
        val replacement = original.copy(content = "new body", revision = 2)
        assertEquals("selected", identity.messagesInOrder(listOf(replacement)).single().content)
    }

    @Test fun temporaryEmptyStreamDoesNotLoseSelection() {
        val row = message("answer", "selected")
        val identity = ChatSelectionIdentity()
        identity.select(row)
        row.content = ""
        assertEquals("selected", identity.messagesInOrder(listOf(row)).single().content)
        row.content = "finished"
        assertTrue(identity.contains(row))
    }

    @Test fun reusableProviderPlaceholdersCannotBeSelected() {
        assertFalse(ChatSelectionIdentity.isSelectable(message("chatgpt:streaming", "loading")))
        assertFalse(ChatSelectionIdentity.isSelectable(message("chatgpt:pending_user", "pending")))
    }

    @Test fun assigningServerIdKeepsLocalSelectionAndLaterRemapping() {
        val row = ChatMessage("user", "hello", createdAtMs = 1)
        val identity = ChatSelectionIdentity()
        identity.select(row)
        val key = identity.key(row)
        row.id = "assigned"
        identity.reconcile(listOf(row))
        val replaced = row.copy(content = "updated")
        assertEquals(key, identity.metadataInOrder(listOf(replaced)).single().identity)
    }

    @Test fun equalIdlessMessagesNeverCollapseIntoOneSelection() {
        val a = ChatMessage("user", "same", createdAtMs = 1)
        val b = a.copy()
        val identity = ChatSelectionIdentity()
        identity.select(b)
        assertNotEquals(identity.key(a), identity.key(b))
        assertFalse(identity.contains(a))
        assertTrue(identity.contains(b))
        assertEquals(1, identity.messagesInOrder(listOf(a, b)).size)
    }

    @Test fun removedOrRecalledSelectionDoesNotMoveToNeighbor() {
        val a = message("a", "first")
        val b = message("b", "second")
        val identity = ChatSelectionIdentity()
        identity.select(a)
        assertTrue(identity.messagesInOrder(listOf(b)).isEmpty())
        identity.select(b)
        b.recalledAt = "now"
        assertTrue(identity.messagesInOrder(listOf(b)).isEmpty())
    }

    @Test fun sourceIdentityAndStreamIdentitySurviveRemapping() {
        val identity = ChatSelectionIdentity()
        val source = ChatMessage("friend", "body", webChatMessage = metadata("provider", "source"))
        val stream = ChatMessage("ai", "stream", streamId = "stream-key")
        identity.select(source)
        identity.select(stream)
        assertEquals(listOf("body", "stream"), identity.messagesInOrder(listOf(
            source.copy(content = "body updated"), stream.copy(content = "stream updated"))).map { it.content })
    }

    @Test fun sameMessageIdInDifferentProvidersIsNotTheSameRow() {
        val a = message("a:id", "a").apply { webChatMessage = metadata("a", "id") }
        val b = message("b:id", "b").apply { webChatMessage = metadata("b", "id") }
        val identity = ChatSelectionIdentity()
        identity.select(a)
        assertFalse(identity.contains(b))
    }

    @Test fun optionalRichMetadataAppearingDoesNotChangeWireIdentity() {
        val row = message("chatgpt_web:message", "body")
        val identity = ChatSelectionIdentity()
        identity.select(row)
        val rebuilt = row.copy(webChatMessage = metadata("chatgpt_web", "message"))
        assertEquals("body", identity.messagesInOrder(listOf(rebuilt)).single().content)
    }

    @Test fun duplicateWireIdsFailClosedInsteadOfSelectingMultipleRows() {
        val row = message("same-id", "original")
        val identity = ChatSelectionIdentity()
        identity.select(row)
        assertTrue(identity.messagesInOrder(listOf(row, row.copy(content = "another"))).isEmpty())
    }

    @Test fun imageOnlyRichMessageIsSelectableAndDeepCopied() {
        val values = mutableListOf(1.0)
        val points = mutableListOf(WebChatProductionRichCard.Point("x", values))
        val parts = mutableListOf(WebChatProductionContentPart("image", "preview"),
            WebChatProductionContentPart("rich_card", "chart", richCard = WebChatProductionRichCard(
                WebChatProductionRichCard.Kind.CHART, "chart", points = points)))
        val row = ChatMessage("friend", "", id = "image", webChatMessage = metadata("p", "image").copy(contentParts = parts))
        assertTrue(ChatSelectionIdentity.isSelectable(row))
        val identity = ChatSelectionIdentity()
        identity.select(row)
        values.add(2.0)
        points.clear()
        parts.clear()
        // Keep a valid live image part while verifying the selected nested lists stay detached.
        row.webChatMessage = metadata("p", "image").copy(contentParts = listOf(WebChatProductionContentPart("image", "new")))
        val frozen = identity.messagesInOrder(listOf(row)).single()
        assertEquals(2, frozen.webChatMessage!!.contentParts.size)
        assertEquals(listOf(1.0), frozen.webChatMessage!!.contentParts[1].richCard!!.points.single().values)
    }

    private fun message(id: String, content: String) = ChatMessage("friend", content, id = id, createdAtMs = 1)
    private fun metadata(provider: String, id: String) = WebChatProductionMessage(provider, id, emptySet())
}
