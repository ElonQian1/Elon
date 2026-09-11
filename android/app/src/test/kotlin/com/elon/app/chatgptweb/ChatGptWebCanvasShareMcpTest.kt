package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasShareMcpTest {
    private val ticket = "sl_" + "17".repeat(16) + "_1"
    private val snapshot = ChatGptWebSnapshot(
        title = "Fixture", url = "https://chatgpt.com/", draft = "preserve",
        messages = emptyList(), authenticated = true, composerReady = false, streaming = false,
        currentModel = "", attachments = emptyList(), dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
    )

    @Test fun nativeConsumerUsesTrackedPrivateManagementWithoutDomOrPublishing() {
        val state = ChatGptWebObservedState()
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_canvas_test"))
        val sent = mutableListOf<JSONObject>()
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onOpenConversation = { fail("must not navigate") }, onInvoke = { fail("must not invoke DOM") },
        ) {
            override fun shareConversation(path: String, requestId: String) { fail("must not publish") }
            override fun manageConversationShares(request: JSONObject, requestId: String) { sent += request }
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { snapshot }, uiManifest = { null }, observedState = state::snapshot,
            beginCommand = state::beginCommand, bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebPresentationMode.NATIVE }, inputText = { "preserve" },
            setInputText = { fail("must not change draft") }, commands = commands,
            refresh = { fail("must not reload") }, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        val consumer = ChatGptWebConsumerPortAdapter({ snapshot }, { null }, state::snapshot, actions::control)
        assertTrue(consumer.manageCanvasShares().accepted)
        assertEquals("list_account", sent.single().getString("operation"))
        assertEquals("canvas", sent.single().getString("resource"))
        assertFalse(sent.single().has("path"))
        assertFalse(consumer.manageCanvasShares(shareId = "fixture", selectionTicket = ticket).accepted)
        assertFalse(consumer.manageCanvasShares(100).accepted)
        assertTrue(consumer.manageCanvasShares(100, ticket).accepted)
        assertEquals(100, sent.last().getInt("offset"))
        assertTrue(consumer.manageCanvasShares(shareId = "fixture", selectionTicket = ticket, userConfirmed = true).accepted)
        assertEquals("revoke_account", sent.last().getString("operation"))
        assertEquals("fixture", sent.last().getString("id"))
        assertEquals(ticket, sent.last().getString("ticket"))
        assertEquals(3, sent.size)
        assertTrue(state.snapshot().commandRequests.all { it.expectedAction == "share_conversation" })
    }

    @Test fun malformedResourceAndConfirmationNeverReachWriteDispatch() {
        var dispatches = 0
        fun request() = JSONObject().put("action", "chatgpt_share_conversation").put("resource", "canvas")
            .put("operation", "revoke_account").put("share_id", "fixture").put("selection_ticket", ticket)
        fun dispatch(args: JSONObject) = ChatGptWebConversationMutationMcpAction.dispatch(
            args, ChatGptWebMcpTestCommandPort(), snapshot,
        ) { _, _ -> dispatches += 1 }
        for (value in listOf<Any>(false, "true", 1, JSONObject.NULL)) {
            assertEquals("user_confirmation_required", dispatch(request().put("user_confirmed", value)))
        }
        for (value in listOf<Any>(123, JSONObject.NULL, "../id")) {
            assertEquals("share_invalid_selection", dispatch(request().put("share_id", value)))
        }
        for (value in listOf<Any>("post", 1, JSONObject.NULL)) {
            assertEquals("share_invalid_selection", dispatch(request().put("resource", value)))
        }
        assertEquals("share_invalid_selection", dispatch(request().put("operation", "revoke")))
        assertEquals("share_invalid_selection", dispatch(request().put("conversation_path", "/c/fixture")))
        assertEquals(0, dispatches)
        assertNull(dispatch(request().put("user_confirmed", true)))
        assertEquals(1, dispatches)
    }
}
