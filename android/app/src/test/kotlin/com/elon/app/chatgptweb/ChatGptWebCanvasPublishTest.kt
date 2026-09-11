package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasPublishTest {
    private val ticket = "sl_" + "17".repeat(16) + "_1"
    private val snapshot = ChatGptWebSnapshot("", "https://chatgpt.com/", "", emptyList(),
        authenticated = true, composerReady = false, streaming = false, currentModel = "",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities(emptySet()))
    private fun input() = JSONObject().put("action", "chatgpt_share_conversation").put("operation", "update_account")
        .put("resource", "canvas").put("share_id", "fixture").put("selection_ticket", ticket)
        .put("user_confirmed", true)

    @Test fun publishRequiresExplicitBooleanConfirmationNotComposerReadiness() {
        var captured: JSONObject? = null
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun manageConversationShares(request: JSONObject, requestId: String) { captured = request }
        }
        val send: (String, (String) -> Unit) -> Unit = { action, dispatch ->
            assertEquals("share_conversation", action); dispatch("mcp_1")
        }
        assertNull(ChatGptWebConversationMutationMcpAction.dispatch(input(), commands, snapshot, dispatchCommand = send))
        assertEquals("update_account", captured?.getString("operation"))
        assertEquals(setOf("operation", "resource", "id", "ticket"), captured?.keys()?.asSequence()?.toSet())
        for (value in listOf(false, "true", 1, JSONObject.NULL)) {
            captured = null
            assertEquals("user_confirmation_required", ChatGptWebConversationMutationMcpAction.dispatch(
                input().put("user_confirmed", value), commands, snapshot, dispatchCommand = send))
            assertNull(captured)
        }
    }

    @Test fun unsafeTargetAndWrongPageDoNotDispatch() {
        var calls = 0
        val send: (String, (String) -> Unit) -> Unit = { _, _ -> calls += 1 }
        val commands = ChatGptWebMcpTestCommandPort()
        for ((key, value) in listOf("share_id" to "../foreign", "selection_ticket" to "expired",
            "resource" to "conversation", "conversation_path" to "/c/11111111-1111-4111-8111-111111111111")) {
            assertNotNull(ChatGptWebConversationMutationMcpAction.dispatch(input().put(key, value), commands,
                snapshot, dispatchCommand = send))
        }
        for (url in listOf("http://chatgpt.com/", "https://chatgpt.com:443/", "https://user@chatgpt.com/", "https://example.com/")) {
            assertEquals("share_context_unavailable", ChatGptWebConversationMutationMcpAction.dispatch(input(), commands,
                snapshot.copy(url = url), dispatchCommand = send))
        }
        assertEquals(0, calls)
    }

    @Test fun nativePortDoesNotInventConfirmationOrIncludeContentInTheRequest() {
        var args: JSONObject? = null
        val port = ChatGptWebConsumerPortAdapter({ null }, { null }, { ChatGptWebObservedState.Snapshot.EMPTY }, { request ->
            args = request; JSONObject().put("control_ok", false)
        })
        port.updateCanvasShare("fixture", ticket, false)
        assertEquals(false, args?.opt("user_confirmed"))
        port.updateCanvasShare("fixture", ticket, true)
        assertEquals(true, args?.opt("user_confirmed"))
        assertEquals("update_account", args?.getString("operation"))
        assertEquals(setOf("action", "operation", "resource", "share_id", "selection_ticket", "user_confirmed"),
            args?.keys()?.asSequence()?.toSet())
        assertEquals("share_canvas_updated", ChatGptWebConversationShareReceipt.detail("share_canvas_updated"))
        assertEquals("share_canvas_update_unconfirmed", ChatGptWebConversationShareReceipt.detail("share_canvas_update_unconfirmed"))
    }
}
