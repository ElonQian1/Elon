package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebDirectoryPageTest {
    private val handle = "dp_" + "a".repeat(32)
    private val next = "dp_" + "b".repeat(32)
    private fun event(id: String = "mcp_page") = JSONObject().put("type", "directory_page").put("version", 1)
        .put("requestId", id).put("scope", "conversations").put("requestedHandle", "").put("handle", handle)
        .put("nextHandle", next).put("complete", false).put("cached", false).put("projects", JSONArray())
        .put("conversations", JSONArray().put(JSONObject().put("id", "fixture").put("title", "Synthetic fixture")
            .put("path", "/c/fixture").put("active", false)))

    private fun parse(event: JSONObject) = ChatGptWebProtocol.parse(JSONObject().put("schema", "yilong.ai.ui.v1")
        .put("adapterVersion", 312).put("documentToken", "doc_directory_fixture").put("providerId", "chatgpt")
        .put("source", "official_web").put("event", event).toString()) as? ChatGptWebEvent.DirectoryPage

    @Test fun pageDataUsesDedicatedStateAndCannotEvictRecentHistory() {
        val observed = ChatGptWebObservedState(nowMs = { 100L })
        val request = observed.beginCommand(ChatGptWebDirectoryPage.ACTION)
        val value = requireNotNull(parse(event(request.id)))
        observed.accept(value)
        observed.accept(ChatGptWebEvent.CommandResult(ChatGptWebDirectoryPage.ACTION, true, "directory_page_ready", request.id))
        assertEquals("fixture", observed.snapshot().directoryPage?.conversations?.single()?.id)
        assertTrue(observed.snapshot().conversations.isEmpty())
        assertEquals(ChatGptWebObservedState.CommandRequest.SUCCEEDED, observed.snapshot().commandRequests.single().status)
    }

    @Test fun olderOrUnrequestedRepliesCannotReplaceTheCurrentPage() {
        val observed = ChatGptWebObservedState(nowMs = { 100L })
        val first = observed.beginCommand(ChatGptWebDirectoryPage.ACTION)
        val second = observed.beginCommand(ChatGptWebDirectoryPage.ACTION)
        observed.accept(requireNotNull(parse(event(first.id))))
        assertNull(observed.snapshot().directoryPage)
        observed.accept(requireNotNull(parse(event(second.id))))
        assertEquals(second.id, observed.snapshot().directoryPage?.requestId)
        observed.clearConversationHistory()
        observed.accept(requireNotNull(parse(event(second.id))))
        assertNull(observed.snapshot().directoryPage)
    }

    @Test fun unknownPageIsNotEmptyCompleteAndMalformedRowsFailRatherThanDisappear() {
        assertFalse(requireNotNull(parse(event().put("nextHandle", JSONObject.NULL))).value.complete)
        assertNull(parse(event().put("complete", true)))
        assertNull(parse(event().put("nextHandle", "raw-private-cursor")))
        assertNull(parse(event().put("requestedHandle", next)))
        val invalid = event()
        invalid.getJSONArray("conversations").getJSONObject(0).put("path", "/c/another")
        assertNull(parse(invalid))
        assertNull(parse(event().put("scope", "g-p-other")))
    }

    @Test fun everyRowAtTheFormerTruncationBoundarySurvives() {
        val rows = JSONArray((196..223).map { n -> JSONObject().put("id", "row-$n")
            .put("title", "Fixture $n").put("path", "/c/row-$n") })
        val result = requireNotNull(parse(event().put("conversations", rows))).value
        assertEquals((196..223).map { "row-$it" }, result.conversations.map { it.id })
    }

    @Test fun aConfirmedEmptyPageAndProjectCatalogAreDistinctFromMissingData() {
        val empty = event().put("nextHandle", JSONObject.NULL).put("complete", true).put("conversations", JSONArray())
        assertTrue(requireNotNull(parse(empty)).value.complete)
        val project = JSONObject().put("id", "g-p-fixture").put("title", "Fixture project").put("path", "/g/g-p-fixture/project")
        val catalog = empty.put("scope", "projects").put("projects", JSONArray().put(project))
        assertEquals("g-p-fixture", requireNotNull(parse(catalog)).value.projects.single().id)
        catalog.remove("projects")
        assertNull(parse(catalog))
    }
}
