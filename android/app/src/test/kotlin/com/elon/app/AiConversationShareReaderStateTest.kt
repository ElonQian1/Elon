package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class AiConversationShareReaderStateTest {
    @Test fun searchIncludesTextBlocksAndCyclesWithoutChangingRows() {
        val rows = listOf(ChatMessage("user", "Alpha", id = "one"), ChatMessage("friend", "", id = "two",
            webChatMessage = WebChatProductionMessage("snapshot", "two", emptySet(), contentParts = listOf(
                WebChatProductionContentPart("code", "example", textBlock = WebChatTextBlock(
                    "block", "code", "example", "kotlin", "ALPHA", true))))))
        val search = AiConversationShareReaderSearch(rows)
        assertEquals(0, search.update("alpha"))
        assertEquals(listOf(0, 1), search.matches)
        assertEquals(1, search.move(-1))
        assertEquals(0, search.move(1))
        assertNull(search.update("absent"))
        assertNull(search.move(1))
        assertEquals("Alpha", rows.first().content)
    }

    @Test fun snapshotsAreDetachedAndGapsAreNotAuthorMessages() {
        val source = ChatMessage("user", "question", id = "q", senderLabel = "Sharer")
        val snapshot = AiConversationShareSnapshot(card(), listOf(source, ChatMessage("friend", "answer", id = "a")), gaps = setOf(1))
        val rows = AiConversationShareReaderPresentation.messages(snapshot)
        source.content = "later edit"
        assertEquals("question", rows[0].content)
        assertEquals("Sharer", rows[0].senderLabel)
        assertEquals("ai-conversation-share-gap", rows[1].role)
        assertNull(rows[1].senderLabel)
        assertFalse(ChatSelectionIdentity.isSelectable(rows[1]))
        assertEquals(3, rows.size)
    }

    @Test fun readerStripsLiveActionsAndRemoteImageSourcesButRetainsLoadingAssetIds() {
        val metadata = WebChatProductionMessage("chatgpt", "source", setOf(WebChatMessageAction.REGENERATE), true,
            listOf(WebChatProductionContentPart("image", "image", assetHandle = "article_media_1",
                imageSource = "https://private.example/image", previewPending = true)))
        val snapshot = AiConversationShareSnapshot(card(), listOf(ChatMessage("friend", "answer",
            modelUsed = "live-model", apkUrl = "https://example.com/app.apk", webChatMessage = metadata)))
        val row = AiConversationShareReaderPresentation.messages(snapshot, pendingImages = true).single()
        assertNull(row.modelUsed)
        assertNull(row.apkUrl)
        assertTrue(row.webChatMessage!!.actions.isEmpty())
        val part = row.webChatMessage!!.contentParts.single()
        assertNull(part.imageSource)
        assertTrue(part.previewPending)
        assertEquals("article_media_1", part.assetHandle)
        assertFalse(AiConversationShareReaderPresentation.messages(snapshot).single()
            .webChatMessage!!.contentParts.single().previewPending)
    }

    @Test fun safePublicLinksRemainAndPrivateOrExecutableLinksAreInert() {
        assertEquals("https://developer.android.com/guide", AiConversationShareReaderLinks.publicUrl("https://developer.android.com/guide"))
        listOf("javascript:alert(1)", "data:text/html,unsafe", "file:///secret", "content://private/image",
            "blob:https://example.com/id", "sandbox:/mnt/data/test", "https://user:secret@example.com/",
            "https://example.com/image?sig=secret", "https://chatgpt.com/backend-api/files/id",
            "https://files.oaiusercontent.com/id", "http://127.0.0.1/file", "http://10.0.0.1/file",
            "http://[::1]/file", "http://localhost/file", "https://host.internal/file",
            "https://example.com/%0Aprivate", "https://example.com/api/files/id")
            .forEach { assertNull(it, AiConversationShareReaderLinks.publicUrl(it)) }
    }

    private fun card() = AiConversationShareCard("ai_snapshot_1", "group", "title", "summary", "chatgpt", "Sharer", 2)
}
