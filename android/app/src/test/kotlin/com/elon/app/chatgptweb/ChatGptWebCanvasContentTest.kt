package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasContentTest {
    private val ticket = "sl_" + "17".repeat(16) + "_1"
    private fun payload(request: String = "mcp_1") = JSONObject()
        .put("type", "canvas_shared_content").put("version", 1).put("requestId", request)
        .put("id", "fixture").put("title", "Synthetic document").put("content", "# Example\n<script>inert()</script>")
        .put("documentType", "document").put("documentVersion", 2).put("access", "public")

    @Test fun displayEventRetainsCompleteTextButNeverEntersShareReceipt() {
        val data = payload().put("content", "x".repeat(128 * 1024))
        val envelope = JSONObject().put("schema", "yilong.ai.ui.v1").put("adapterVersion", 358)
            .put("documentToken", "doc_fixture").put("event", data)
        val event = ChatGptWebProtocol.parse(envelope.toString(), 358) as ChatGptWebEvent.CanvasContent
        assertEquals(128 * 1024, event.value.content.length)
        assertEquals(2L, event.value.documentVersion)
        assertFalse(event.value.isCode)
        assertFalse(event.value.toString().contains("xxxx"))
        assertEquals(setOf("request_id", "content_length", "document_type", "document_version"),
            event.value.diagnostic().keys().asSequence().toSet())
        assertEquals("share_list_unconfirmed", ChatGptWebConversationShareReceipt.detail(data.toString()))
        assertEquals("share_canvas_ready", ChatGptWebConversationShareReceipt.detail("share_canvas_ready"))
        assertNull(ChatGptWebProtocol.parse(envelope.put("adapterVersion", 357).toString(), 358))
    }

    @Test fun optionalVersionAndCodeRemainReadOnlySource() {
        val value = requireNotNull(ChatGptWebCanvasContent.parse(payload()
            .put("documentVersion", JSONObject.NULL).put("documentType", "webview")))
        assertNull(value.documentVersion)
        assertTrue(value.isCode)
        assertTrue(value.content.contains("<script>"))
    }

    @Test fun strictDisplaySchemaRejectsCoercionForeignScopesAndTruncation() {
        for ((key, value) in listOf(
            "type" to "snapshot", "version" to "1", "requestId" to "mcp_invalid!", "id" to "../other",
            "title" to "", "title" to "bad\nname", "title" to "x".repeat(513), "content" to JSONObject.NULL,
            "content" to "x".repeat(128 * 1024 + 1), "documentType" to "unknown",
            "documentVersion" to 0, "documentVersion" to "2", "documentVersion" to 2.5,
            "access" to "workspace", "headers" to "not-allowed",
        )) assertNull("invalid: $key", ChatGptWebCanvasContent.parse(payload().put(key, value)))
    }

    @Test fun readRequiresCanvasSelectionButNotComposerOrMutationConfirmation() {
        var request: JSONObject? = null
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun manageConversationShares(value: JSONObject, requestId: String) { request = value }
        }
        val snapshot = ChatGptWebSnapshot("", "https://chatgpt.com/", "", emptyList(),
            authenticated = true, composerReady = false, streaming = false, currentModel = "",
            attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities(emptySet()))
        val args = JSONObject().put("action", "chatgpt_share_conversation").put("operation", "read_account")
            .put("resource", "canvas").put("share_id", "fixture").put("selection_ticket", ticket)
        val dispatch: (String, (String) -> Unit) -> Unit = { action, send ->
            assertEquals("share_conversation", action); send("mcp_1")
        }
        assertNull(ChatGptWebConversationMutationMcpAction.dispatch(args, commands, snapshot, dispatchCommand = dispatch))
        assertEquals("read_account", request?.getString("operation"))
        assertFalse(requireNotNull(request).has("path"))
        for ((key, value) in listOf("resource" to "conversation", "share_id" to "../x", "selection_ticket" to "expired",
            "conversation_path" to "/c/11111111-1111-4111-8111-111111111111")) {
            request = null
            assertNotNull(ChatGptWebConversationMutationMcpAction.dispatch(JSONObject(args.toString()).put(key, value),
                commands, snapshot, dispatchCommand = dispatch))
            assertNull(request)
        }
    }

    @Test fun lateSupersededOrClearedContentCannotResurrectTheViewer() {
        val state = ChatGptWebObservedState()
        val document = WebBridgeDocumentSession()
        val page = document.beginPage()
        state.updateDocument(requireNotNull(document.accept(page.documentToken)))
        val first = state.beginCommand(ChatGptWebCanvasContent.ACTION)
        state.accept(ChatGptWebEvent.CanvasContent(requireNotNull(ChatGptWebCanvasContent.parse(payload(first.id)))))
        assertNotNull(state.snapshot().canvasContent)
        val next = state.beginCommand(ChatGptWebCanvasContent.ACTION)
        assertNull(state.snapshot().canvasContent)
        state.accept(ChatGptWebEvent.CanvasContent(requireNotNull(ChatGptWebCanvasContent.parse(payload(first.id)))))
        assertNull(state.snapshot().canvasContent)
        state.accept(ChatGptWebEvent.CanvasContent(requireNotNull(ChatGptWebCanvasContent.parse(payload(next.id)))))
        assertEquals(next.id, state.snapshot().canvasContent?.requestId)
        state.clearConversationHistory()
        state.accept(ChatGptWebEvent.CanvasContent(requireNotNull(ChatGptWebCanvasContent.parse(payload(next.id)))))
        assertNull(state.snapshot().canvasContent)
        val third = state.beginCommand(ChatGptWebCanvasContent.ACTION)
        state.accept(ChatGptWebEvent.CanvasContent(requireNotNull(ChatGptWebCanvasContent.parse(payload(third.id)))))
        state.updateDocument(document.beginPage())
        assertNull(state.snapshot().canvasContent)
        state.accept(ChatGptWebEvent.CanvasContent(requireNotNull(ChatGptWebCanvasContent.parse(payload(third.id)))))
        assertNull(state.snapshot().canvasContent)
    }

    @Test fun productionPortDispatchesReadOnlyAndOnlyExposesCurrentDisplayData() {
        val value = requireNotNull(ChatGptWebCanvasContent.parse(payload()))
        var observed = ChatGptWebObservedState.Snapshot.EMPTY.copy(pageGeneration = 1, adapterGeneration = 1, canvasContent = value)
        var args: JSONObject? = null
        val port = ChatGptWebConsumerPortAdapter({ null }, { null }, { observed }, { request ->
            args = request; JSONObject().put("control_ok", false)
        })
        port.readCanvasShare("fixture", ticket)
        assertEquals("read_account", args?.getString("operation"))
        assertEquals("canvas", args?.getString("resource"))
        assertEquals(ticket, args?.getString("selection_ticket"))
        assertFalse(requireNotNull(args).has("user_confirmed"))
        assertEquals(value, port.canvasContent())
        observed = observed.copy(pageGeneration = 2)
        assertNull(port.canvasContent())
    }
}
