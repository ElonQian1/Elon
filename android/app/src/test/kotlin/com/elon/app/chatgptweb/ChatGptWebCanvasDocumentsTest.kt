package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasDocumentsTest {
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val token = "cd_" + "17".repeat(16) + "_1"
    private fun document() = JSONObject().put("id", "synthetic").put("title", "Synthetic")
        .put("content", "abc").put("documentType", "document").put("documentVersion", 4)
        .put("comments", JSONArray())
    private fun payload(request: String = "mcp_1") = JSONObject().put("type", "canvas_documents").put("version", 1)
        .put("requestId", request).put("path", path).put("ticket", token).put("scope", token)
        .put("documents", JSONArray().put(document())).put("unconfirmedWrite", false)
    private fun request() = JSONObject().put("operation", "save").put("path", path).put("ticket", token).put("scope", token)
        .put("id", "synthetic").put("content", "Edited").put("comments", JSONArray())

    @Test fun fullTypedEventAndDiagnosticAreSeparate() {
        val raw = payload().put("documents", JSONArray().put(document().put("content", "x".repeat(128 * 1024))))
        val envelope = JSONObject().put("schema", "yilong.ai.ui.v1").put("adapterVersion", 359)
            .put("documentToken", "doc_fixture").put("event", raw)
        val event = ChatGptWebProtocol.parse(envelope.toString(), 359) as ChatGptWebEvent.CanvasDocuments
        assertEquals(128 * 1024, event.value.documents.single().content.length)
        assertEquals(setOf("request_id", "document_count", "unconfirmed_write"), event.value.diagnostic().keys().asSequence().toSet())
        assertFalse(event.value.toString().contains(path))
        assertFalse(event.value.documents.single().toString().contains("xxxx"))
    }

    @Test fun strictSchemaRejectsCoercionInvalidScopeDuplicatesAndPartialText() {
        for ((key, value) in listOf("version" to "1", "scope" to "expired", "requestId" to "foreign",
            "path" to "$path?other=1", "unconfirmedWrite" to "false", "secret" to "rejected"))
            assertNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put(key, value)))
        for ((key, value) in listOf("content" to "\uD800", "content" to "x".repeat(128 * 1024 + 1),
            "documentVersion" to "4", "documentVersion" to 0, "documentType" to "unknown", "title" to "bad\nline"))
            assertNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("documents", JSONArray().put(document().put(key, value)))))
        assertNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("documents", JSONArray().put(document()).put(document()))))
        val comment = JSONObject().put("id", "comment").put("start", 1).put("end", 2).put("content", "fixture")
        assertNull(ChatGptWebCanvasDocumentProtocol.parse(payload().put("documents", JSONArray().put(document()
            .put("content", "\uD83D\uDE00x").put("comments", JSONArray().put(comment))))))
    }

    @Test fun canonicalSaveRequiresStrictConfirmationAndPreservesFullBody() {
        var submitted: JSONObject? = null
        var confirmation = false
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun canvasDocument(request: JSONObject, confirmed: Boolean, requestId: String) {
                assertEquals("mcp_test", requestId); submitted = request; confirmation = confirmed
            }
        }
        val dispatch: (String, (String) -> Unit) -> Unit = { action, send ->
            assertEquals("canvas_document", action); send("mcp_test")
        }
        val args = JSONObject().put("canvas_request", request()).put("user_confirmed", false)
        assertEquals("canvas_confirmation_required", ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
        assertNull(submitted)
        args.put("user_confirmed", "true")
        assertEquals("canvas_confirmation_required", ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
        args.put("user_confirmed", true)
        assertNull(ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
        assertTrue(confirmation)
        assertEquals("Edited", submitted?.getString("content"))
        for ((key, value) in listOf("extra" to true, "scope" to "bad", "content" to "\uD800"))
            assertNull(ChatGptWebCanvasDocumentProtocol.request(request().put(key, value)))
    }

    @Test fun dataIsBoundToTheLatestRequestAndDocumentGeneration() {
        val state = ChatGptWebObservedState()
        val doc = WebBridgeDocumentSession()
        val page = doc.beginPage()
        state.updateDocument(requireNotNull(doc.accept(page.documentToken)))
        fun send(id: String) = state.accept(ChatGptWebEvent.CanvasDocuments(requireNotNull(ChatGptWebCanvasDocumentProtocol.parse(payload(id)))))
        val first = state.beginCommand("canvas_document"); send(first.id)
        assertNotNull(state.snapshot().canvasDocuments)
        val next = state.beginCommand("canvas_document"); send(first.id)
        assertNull(state.snapshot().canvasDocuments)
        send(next.id)
        assertEquals(next.id, state.snapshot().canvasDocuments?.requestId)
        state.clearConversationHistory(); send(next.id)
        assertNull(state.snapshot().canvasDocuments)
        val last = state.beginCommand("canvas_document"); send(last.id)
        state.updateDocument(doc.beginPage()); send(last.id)
        assertNull(state.snapshot().canvasDocuments)
    }

    @Test fun renameRequiresExactSchemaAndExplicitConfirmation() {
        fun rename(title: Any = "New title") = JSONObject().put("operation", "rename")
            .put("path", path).put("ticket", token).put("scope", token).put("id", "synthetic").put("title", title)
        for (title in listOf("", " ", " padded", "trailing ", "line\nfeed", "\u0000", "x".repeat(513), "\uD800", 5))
            assertNull(ChatGptWebCanvasDocumentProtocol.request(rename(title)))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(rename().put("content", "Not part of rename")))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(rename().put("documentVersion", 4)))
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(rename("中文\uD83D\uDE00")))
        var submitted: JSONObject? = null
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun canvasDocument(request: JSONObject, confirmed: Boolean, requestId: String) {
                assertTrue(confirmed)
                assertEquals("mcp_rename", requestId)
                submitted = request
            }
        }
        val dispatch: (String, (String) -> Unit) -> Unit = { action, send ->
            assertEquals("canvas_document", action); send("mcp_rename")
        }
        val args = JSONObject().put("canvas_request", rename())
        for (confirmation in listOf(false, "true")) {
            args.put("user_confirmed", confirmation)
            assertEquals("canvas_confirmation_required", ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
            assertNull(submitted)
        }
        args.put("user_confirmed", true)
        assertNull(ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
        assertEquals("New title", submitted?.getString("title"))
        assertFalse(requireNotNull(submitted).has("content"))
    }

    @Test fun productionPortPreservesSelectionAndDoesNotRequireComposer() {
        val value = requireNotNull(ChatGptWebCanvasDocumentProtocol.parse(payload()))
        var state = ChatGptWebObservedState.Snapshot.EMPTY.copy(pageGeneration = 1, adapterGeneration = 1, canvasDocuments = value)
        var args: JSONObject? = null
        val port = ChatGptWebConsumerPortAdapter({ null }, { null }, { state }, { input ->
            args = input; JSONObject().put("control_ok", false)
        })
        port.canvasDocument(request(), true)
        assertEquals("chatgpt_canvas_document", args?.getString("action"))
        assertTrue(requireNotNull(args).getBoolean("user_confirmed"))
        assertEquals(token, args?.getJSONObject("canvas_request")?.getString("scope"))
        assertEquals(value, port.canvasDocuments())
        state = state.copy(pageGeneration = 2)
        assertNull(port.canvasDocuments())
        assertNull(ChatGptWebOperationReadiness.rejection("chatgpt_canvas_document", null, adapterCurrent = true, bridgeReady = false))
    }

    @Test fun commentDismissalCannotCarryAcceptReasonOrAReplacementBody() {
        fun dismiss(comment: Any = "c") = JSONObject().put("operation", "dismiss_comment").put("path", path)
            .put("ticket", token).put("scope", token).put("id", "synthetic").put("commentId", comment)
        for (comment in listOf("", "../foreign", "c?reason=accept", 5))
            assertNull(ChatGptWebCanvasDocumentProtocol.request(dismiss(comment)))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(dismiss().put("reason", "accept")))
        assertNull(ChatGptWebCanvasDocumentProtocol.request(dismiss().put("content", "replacement")))
        var submitted: JSONObject? = null
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun canvasDocument(request: JSONObject, confirmed: Boolean, requestId: String) {
                assertTrue(confirmed)
                submitted = request
            }
        }
        val dispatch: (String, (String) -> Unit) -> Unit = { _, send -> send("mcp_dismiss") }
        val args = JSONObject().put("canvas_request", dismiss())
        for (confirmation in listOf(false, "true")) {
            args.put("user_confirmed", confirmation)
            assertEquals("canvas_confirmation_required", ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
            assertNull(submitted)
        }
        args.put("user_confirmed", true)
        assertNull(ChatGptWebCanvasDocumentProtocol.dispatch(args, commands, dispatch))
        assertEquals("dismiss_comment", submitted?.getString("operation"))
        assertEquals("c", submitted?.getString("commentId"))
    }
}
