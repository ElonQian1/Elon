package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatConversationFileIndex
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebConversationFilesTest {
    @Test fun consumesExactPrivateSourceLinkProducerFixture() {
        val fixture = JSONObject(requireNotNull(javaClass.classLoader?.getResourceAsStream(
            "webchat/private-source-links-contract.json")).bufferedReader().use { it.readText() }).getJSONObject("event")
        val parsed = (ChatGptWebProtocol.parse(JSONObject().put("schema", "yilong.ai.ui.v1")
            .put("event", fixture).toString()) as ChatGptWebEvent.ConversationFiles).value
        assertEquals(listOf("first", "second"), parsed.files.map { it.name })
        assertEquals(listOf("https://docs.example.test/document/first#part",
            "https://docs.example.test/document/second?tab=one"), parsed.files.map { it.sourceUrl })
        assertTrue(parsed.files.all { it.kind == "source" && it.downloadHandle.isEmpty() })
        assertFalse(parsed.truncated)
    }

    @Test fun sourceRowsCarryOnlyAnExplicitSafeLinkAndCannotAdvertiseDownload() {
        val source = fixture()
        val row = source.getJSONArray("files").getJSONObject(0)
        row.put("kind", "source").put("sourceUrl", "https://docs.example.test/document/fixture#section")
        val parsed = requireNotNull(ChatGptWebConversationFiles.parse(source))
        assertEquals("source", parsed.files[0].kind)
        assertEquals("https://docs.example.test/document/fixture#section", parsed.files[0].sourceUrl)
        assertEquals("", parsed.files[0].downloadHandle)
        assertFalse(parsed.truncated)
        for (url in listOf("javascript:alert(1)", "https://user:pass@docs.example.test/", "")) {
            row.put("sourceUrl", url)
            val rejected = requireNotNull(ChatGptWebConversationFiles.parse(source))
            assertTrue(rejected.truncated)
            assertTrue(rejected.files.none { it.kind == "source" })
        }
        row.put("sourceUrl", "https://docs.example.test/document/fixture")
            .put("downloadHandle", "download_" + "a".repeat(32))
        assertTrue(requireNotNull(ChatGptWebConversationFiles.parse(source)).files.none { it.kind == "source" })
    }

    @Test fun downloadSelectionAcceptsOnlyOpaqueHandlesWithoutChangingExistingDescriptors() {
        val source = fixture()
        val row = source.getJSONArray("files").getJSONObject(0)
        row.put("downloadHandle", "download_" + "a".repeat(32))
        assertEquals("download_" + "a".repeat(32), requireNotNull(ChatGptWebConversationFiles.parse(source)).files[0].downloadHandle)
        row.put("downloadHandle", "https://files.oaiusercontent.com/a?sig=secret")
        assertEquals("", requireNotNull(ChatGptWebConversationFiles.parse(source)).files[0].downloadHandle)
    }
    private fun fixture(): JSONObject = JSONObject(requireNotNull(javaClass.classLoader?.getResourceAsStream(
        "webchat/private-conversation-files-contract.json")).bufferedReader().use { it.readText() }).getJSONObject("event")

    private fun index(path: String = "/c/fixture", requestId: String = "mcp_f1"): WebChatConversationFileIndex =
        requireNotNull(ChatGptWebConversationFiles.parse(fixture().put("conversationPath", path).put("requestId", requestId)))

    @Test fun consumesExactProducerFixtureThroughRealBridgeParser() {
        val payload = JSONObject().put("schema", "yilong.ai.ui.v1").put("event", fixture())
        val result = (ChatGptWebProtocol.parse(payload.toString()) as ChatGptWebEvent.ConversationFiles).value
        assertEquals("/c/fixture", result.path)
        assertEquals("mcp_f1", result.requestId)
        assertEquals(listOf("fixture.txt", "图片"), result.files.map { it.name })
        assertEquals("text/plain", result.files[0].mediaType)
        assertEquals("assistant-1", result.files[1].messageId)
        assertFalse(result.truncated)
    }

    @Test fun parserRejectsUnknownResponseAndUnsafePathOrRequest() {
        assertNull(ChatGptWebConversationFiles.parse(fixture().put("conversationPath", "https://example.com/c/test")))
        assertNull(ChatGptWebConversationFiles.parse(fixture().put("requestId", "unbound")))
        assertNull(ChatGptWebConversationFiles.parse(fixture().apply { remove("files") }))
    }

    @Test fun malformedOrOversizedDescriptorsArePartialNotEmptySuccess() {
        val malformed = fixture()
        malformed.getJSONArray("files").getJSONObject(0).put("messageId", "../../invalid")
        val result = requireNotNull(ChatGptWebConversationFiles.parse(malformed))
        assertEquals(1, result.files.size)
        assertTrue(result.truncated)
        val large = fixture()
        val row = large.getJSONArray("files").getJSONObject(0)
        val rows = org.json.JSONArray()
        repeat(101) { rows.put(JSONObject(row.toString()).put("id", "msg-$it:0").put("messageId", "msg-$it")) }
        large.put("files", rows)
        val bounded = requireNotNull(ChatGptWebConversationFiles.parse(large))
        assertEquals(100, bounded.files.size)
        assertTrue(bounded.truncated)
    }

    @Test fun cacheIsBoundedByCanonicalConversationAndHasShortFreshness() {
        val cache = ChatGptWebConversationFileCache()
        cache.accept(index(), 1_000)
        cache.accept(index("/g/g-p-demo/c/fixture", "mcp_new"), 2_000)
        assertEquals(1, cache.snapshot().size)
        val value = cache.snapshot().getValue("fixture")
        assertEquals("mcp_new", value.requestId)
        assertTrue(value.isFresh(61_999))
        assertFalse(value.isFresh(62_000))
        assertFalse(value.isFresh(1_999))
        repeat(9) { cache.accept(index("/c/test-$it"), 3_000 + it.toLong()) }
        assertEquals(8, cache.snapshot().size)
        assertFalse(cache.snapshot().containsKey("fixture"))
        cache.clear()
        assertTrue(cache.snapshot().isEmpty())
    }

    @Test fun latestMatchingRequestOwnsIndexWithoutChangingConversationSnapshot() {
        val state = ChatGptWebObservedState(nowMs = { 1_000 })
        val old = state.beginConversationFilesCommand("/c/fixture")
        val latest = state.beginConversationFilesCommand("/g/g-p-demo/c/fixture")
        state.accept(ChatGptWebEvent.ConversationFiles(index(requestId = old.id)))
        assertTrue(state.snapshot().conversationFiles.isEmpty())
        state.accept(ChatGptWebEvent.ConversationFiles(index("/c/other", latest.id)))
        assertTrue(state.snapshot().conversationFiles.isEmpty())
        state.accept(ChatGptWebEvent.ConversationFiles(index(requestId = latest.id)))
        assertEquals(latest.id, state.snapshot().conversationFiles.getValue("fixture").requestId)
        assertTrue(state.snapshot().conversations.isEmpty())
        assertTrue(state.snapshot().composerSections.isEmpty())
    }

    @Test fun timeoutOrDocumentReplacementRejectsLateIndex() {
        var now = 1_000L
        val state = ChatGptWebObservedState(nowMs = { now })
        val expired = state.beginConversationFilesCommand("/c/fixture")
        now += 20_001
        state.accept(ChatGptWebEvent.ConversationFiles(index(requestId = expired.id)))
        assertTrue(state.snapshot().conversationFiles.isEmpty())
        val replaced = state.beginConversationFilesCommand("/c/fixture")
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 0, "doc_test"))
        state.accept(ChatGptWebEvent.ConversationFiles(index(requestId = replaced.id)))
        assertTrue(state.snapshot().conversationFiles.isEmpty())
    }

    @Test fun clearingAccountHistoryCannotBeUndoneByPendingResponse() {
        val state = ChatGptWebObservedState(nowMs = { 1_000 })
        val first = state.beginConversationFilesCommand("/c/fixture")
        state.accept(ChatGptWebEvent.ConversationFiles(index(requestId = first.id)))
        val late = state.beginConversationFilesCommand("/c/other")
        state.clearConversationHistory()
        state.accept(ChatGptWebEvent.ConversationFiles(index("/c/other", late.id)))
        assertTrue(state.snapshot().conversationFiles.isEmpty())
    }

    @Test fun failedRefreshPreservesPriorCachedIndex() {
        val state = ChatGptWebObservedState(nowMs = { 1_000 })
        val first = state.beginConversationFilesCommand("/c/fixture")
        state.accept(ChatGptWebEvent.ConversationFiles(index(requestId = first.id)))
        val retry = state.beginConversationFilesCommand("/c/fixture")
        state.accept(ChatGptWebEvent.CommandResult(ChatGptWebConversationFiles.ACTION, false, "files_read_failed", retry.id))
        assertEquals(first.id, state.snapshot().conversationFiles.getValue("fixture").requestId)
    }
}
