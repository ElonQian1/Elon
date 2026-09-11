package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasManagementTest {
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val token = "cd_" + "17".repeat(16) + "_1"
    private fun document(version: Int) = JSONObject().put("id", "synthetic").put("title", "Synthetic")
        .put("content", "<script>inert</script>\n" + "x".repeat(40000)).put("documentType", "code/javascript")
        .put("documentVersion", version).put("comments", JSONArray())
    private fun history() = JSONObject().put("documentId", "synthetic").put("ticket", token).put("beforeVersion", 4)
        .put("nextBeforeVersion", 2).put("versions", JSONArray().put(document(3)).put(document(2)))
    private fun share(state: String = "public") = JSONObject().put("documentId", "synthetic").put("state", state)
        .put("id", if (state == "public") "synthetic_shared" else "")
        .put("documentVersion", if (state == "public") 3 else JSONObject.NULL)
    private fun payload() = JSONObject().put("type", "canvas_documents").put("version", 1).put("requestId", "mcp_1")
        .put("path", path).put("ticket", token).put("scope", token).put("unconfirmedWrite", false)
        .put("documents", JSONArray().put(document(4)))
    private fun selection(operation: String) = JSONObject().put("operation", operation).put("path", path)
        .put("ticket", token).put("scope", token).put("id", "synthetic")

    @Test fun historyPreservesInertFullBodyAndPagination() {
        val value = requireNotNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("history", history())))
        val page = requireNotNull(value.history)
        assertEquals(4L, page.beforeVersion)
        assertEquals(2L, page.nextBeforeVersion)
        assertEquals(listOf(3L, 2L), page.versions.map { it.documentVersion })
        assertEquals(document(3).getString("content"), page.versions[0].content)
        assertFalse(page.toString().contains("script"))
    }

    @Test fun historyRejectsForeignOwnersDuplicateVersionsAndInvalidCursor() {
        val invalid = listOf(history().put("documentId", "other"), history().put("ticket", "bad"),
            history().put("beforeVersion", "4"), history().put("beforeVersion", 5), history().put("nextBeforeVersion", 1),
            history().put("versions", JSONArray().put(document(3)).put(document(3))),
            history().put("versions", JSONArray().put(document(2)).put(document(3))),
            history().put("versions", JSONArray().put(document(4))), history().put("extra", true))
        invalid.forEach { assertNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("history", it))) }
        val empty = history().put("versions", JSONArray()).put("nextBeforeVersion", JSONObject.NULL)
        assertNotNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("history", empty)))
    }

    @Test fun sharesExposeOnlyStrictPublicLinksAndKeepOlderPublishedVersion() {
        val value = requireNotNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("share", share())))
        assertEquals("https://chatgpt.com/canvas/shared/synthetic_shared", value.share?.url)
        assertEquals(3L, value.share?.documentVersion)
        for (state in listOf("missing", "restricted")) {
            val result = requireNotNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("share", share(state))))
            assertNull(result.share?.url)
        }
        assertFalse(value.share.toString().contains("synthetic"))
        assertEquals(setOf("request_id", "document_count", "unconfirmed_write"), value.diagnostic().keys().asSequence().toSet())
    }

    @Test fun malformedShareIsNotRenderedAsMissingOrCopyable() {
        for (invalid in listOf(share().put("id", "../foreign"), share().put("state", "unknown"), share().put("documentId", "other"),
            share().put("documentVersion", "3"), share().put("documentVersion", JSONObject.NULL), share().put("url", "https://other/"),
            share("missing").put("id", "not_empty"), share("restricted").put("documentVersion", 3))) {
            assertNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("share", invalid)))
        }
    }

    @Test fun historyAndRestoreRequestsHaveExactSelectionAndVersionSchemas() {
        val history = selection("history").put("beforeVersion", 4)
        val restore = selection("restore").put("historyTicket", token).put("restoreVersion", 3)
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(history))
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(restore))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(history.put("beforeVersion", "4")))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(restore.put("historyTicket", "stale")))
        for (operation in listOf("share_lookup", "share_create", "share_ack")) {
            assertNotNull(ChatGptWebCanvasDocumentProtocol.request(selection(operation)))
            assertNull(ChatGptWebCanvasDocumentProtocol.request(selection(operation).put("access", "public")))
        }
    }

    @Test fun publicationRestorationAndAcknowledgementRequireExplicitUserConfirmation() {
        val commands = ChatGptWebMcpTestCommandPort()
        var count = 0
        val dispatch: (String, (String) -> Unit) -> Unit = { action, _ -> assertEquals("canvas_document", action); count += 1 }
        for (operation in listOf("restore", "share_create", "share_ack")) {
            val request = selection(operation)
            if (operation == "restore") request.put("historyTicket", token).put("restoreVersion", 3)
            val args = JSONObject().put("canvas_request", request).put("user_confirmed", false)
            assertEquals("canvas_confirmation_required", ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
            assertNull(ChatGptWebCanvasDocumentProtocol.dispatch(args.put("user_confirmed", true), commands, dispatch))
        }
        assertEquals(3, count)
    }
}
