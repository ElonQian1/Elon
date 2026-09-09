package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatConsumerCommandStatus
import com.elon.app.WebChatConversationSharePolicy
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebConversationShareMcpTest {
    @Test fun accountManagementRejectsInvalidScopeAndPageBeforeDispatch() {
        val snapshot = ChatGptWebSnapshot(
            title = "Fixture", url = "https://chatgpt.com/", draft = "unsent",
            messages = emptyList(), authenticated = true, composerReady = false, streaming = false,
            currentModel = "", attachments = emptyList(), dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY,
        )
        val commands = ChatGptWebMcpTestCommandPort()
        var dispatches = 0
        fun request() = JSONObject().put("action", "chatgpt_share_conversation").put("operation", "list_account")
        fun dispatch(args: JSONObject, page: ChatGptWebSnapshot? = snapshot) =
            ChatGptWebConversationMutationMcpAction.dispatch(args, commands, page) { _, _ -> dispatches += 1 }
        assertNull(dispatch(request()))
        assertEquals(1, dispatches)
        for (offset in listOf<Any>(-100, 1, 1000, "100", 100.0, JSONObject.NULL)) {
            assertEquals("share_invalid_selection", dispatch(request().put("page_offset", offset)))
        }
        assertEquals("share_invalid_selection", dispatch(request().put("page_offset", 100)))
        assertEquals("share_invalid_selection", dispatch(request().put("selection_ticket", JSONObject.NULL)))
        assertEquals("share_invalid_selection", dispatch(request().put("conversation_path", "/c/fixture")))
        assertEquals("share_invalid_selection", dispatch(request().put("share_id", "all")))
        for (url in listOf("https://example.com/", "http://chatgpt.com/", "https://chatgpt.com:8443/", "https://user@chatgpt.com/")) {
            assertEquals("share_context_unavailable", dispatch(request(), snapshot.copy(url = url)))
        }
        assertEquals("share_context_unavailable", dispatch(request(), null))
        assertEquals(1, dispatches)
    }

    @Test fun managementUsesTrackedReadAndConfirmedRevokeWithoutNavigatingOrPublishing() {
        val id = "44444444-4444-4444-8444-444444444444"
        val path = "/c/$id"
        val state = ChatGptWebObservedState()
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_share_test"))
        val snapshot = ChatGptWebSnapshot(
            title = "Fixture", url = "https://chatgpt.com/", draft = "preserve",
            messages = emptyList(), authenticated = true, composerReady = false, streaming = false,
            currentModel = "", attachments = emptyList(), dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY,
        )
        val sent = mutableListOf<JSONObject>()
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onOpenConversation = { fail("management must not navigate") },
            onInvoke = { fail("management must not invoke DOM") },
        ) {
            override fun shareConversation(path: String, requestId: String) { fail("must not publish") }
            override fun manageConversationShares(request: JSONObject, requestId: String) { sent += request }
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { snapshot }, uiManifest = { null }, observedState = state::snapshot,
            beginCommand = state::beginCommand, bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebPresentationMode.NATIVE }, inputText = { "preserve native draft" },
            setInputText = { fail("must not change draft") }, commands = commands,
            refresh = { fail("must not reload") }, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        val consumer = ChatGptWebConsumerPortAdapter({ snapshot }, { null }, state::snapshot, actions::control)
        assertTrue(consumer.manageConversationShares(path).accepted)
        assertEquals("list", sent.single().getString("operation"))
        val ticket = "sl_" + "17".repeat(16) + "_1"
        assertFalse(consumer.manageConversationShares(path, id, ticket, false).accepted)
        assertFalse(consumer.manageConversationShares(path, "all", ticket, true).accepted)
        assertTrue(consumer.manageConversationShares(path, id, ticket, true).accepted)
        assertEquals(listOf("list", "revoke"), sent.map { it.getString("operation") })
        assertTrue(consumer.manageAccountShares().accepted)
        assertEquals("list_account", sent.last().getString("operation"))
        assertFalse(sent.last().has("path"))
        assertTrue(consumer.manageAccountShares(100, ticket).accepted)
        assertEquals(100, sent.last().getInt("offset"))
        assertEquals(ticket, sent.last().getString("ticket"))
        assertFalse(consumer.manageAccountShares(100).accepted)
        assertFalse(consumer.manageAccountShares(1, ticket).accepted)
        assertTrue(state.snapshot().commandRequests.all { it.expectedAction == "share_conversation" })
    }

    @Test fun projectMemberIntentUsesTheSameTrackedCommandWithoutPublicScopeConversion() {
        val id = "44444444-4444-4444-8444-444444444444"
        val project = "g-p-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        val path = "/g/$project/c/$id"
        val snapshot = ChatGptWebSnapshot(
            title = "Synthetic member fixture", url = "https://chatgpt.com/c/$id", draft = "unsent",
            messages = emptyList(), authenticated = true, composerReady = true, streaming = false,
            currentModel = "", attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
        )
        var dispatched = 0
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onOpenConversation = { fail("share dispatch must not navigate") },
            onInvoke = { fail("member share must not invoke DOM") },
        ) {
            override fun shareConversation(path: String, requestId: String) {
                assertEquals("/g/$project/c/$id", path)
                assertEquals("req_members", requestId)
                dispatched += 1
            }
        }
        val args = JSONObject().put("action", "chatgpt_share_conversation")
            .put("conversation_path", path).put("user_confirmed", true)
        fun dispatch(value: ChatGptWebSnapshot) = ChatGptWebConversationMutationMcpAction.dispatch(args, commands, value) { action, send ->
            assertEquals("share_conversation", action)
            send("req_members")
        }
        assertNull(dispatch(snapshot))
        assertNull(dispatch(snapshot.copy(url = "https://chatgpt.com/g/$project-fixture/c/$id")))
        assertEquals("share_context_unavailable", dispatch(snapshot.copy(url = "https://example.com/c/$id")))
        assertEquals("share_conversation_busy", dispatch(snapshot.copy(streaming = true)))
        args.put("user_confirmed", false)
        assertEquals("user_confirmation_required", dispatch(snapshot))
        assertEquals(2, dispatched)
    }

    @Test fun nativeConsumerSharesThroughTheTrackedPrivateCommandOnlyAfterConfirmation() {
        val state = ChatGptWebObservedState()
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_share_test"))
        var sent = ""
        var sentRequest = ""
        var snapshot = ChatGptWebSnapshot(
            title = "Synthetic fixture", url = "https://chatgpt.com/c/fixture", draft = "unsent fixture",
            messages = emptyList(), authenticated = true, composerReady = true, streaming = false,
            currentModel = "", attachments = emptyList(), dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY,
        )
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onOpenConversation = { fail("share dispatch must not navigate") },
            onInvoke = { fail("share must not invoke a DOM control") },
            onSetDraft = { _, _ -> fail("share must not change a draft") },
        ) {
            override fun shareConversation(path: String, requestId: String) {
                sent = path
                sentRequest = requestId
            }
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { snapshot }, uiManifest = { null }, observedState = state::snapshot,
            beginCommand = state::beginCommand, bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebPresentationMode.NATIVE }, inputText = { "unsent native fixture" },
            setInputText = { fail("share must not change input") }, commands = commands,
            refresh = { fail("share must not reload") }, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        val consumer = ChatGptWebConsumerPortAdapter({ snapshot }, { null }, state::snapshot, actions::control)
        assertFalse(consumer.shareConversation("/c/fixture", false).accepted)
        assertFalse(consumer.shareConversation("/c/other", true).accepted)
        assertEquals("", sent)
        val result = consumer.shareConversation("/c/fixture", true)
        assertTrue(result.accepted)
        assertEquals("/c/fixture", sent)
        assertEquals(result.requestId, sentRequest)
        assertEquals("share_conversation", state.snapshot().commandRequests.single().expectedAction)
        val url = "https://chatgpt.com/share/44444444-4444-4444-8444-444444444444"
        state.accept(ChatGptWebEvent.CommandResult("share_conversation", true, "share_link_ready:$url", result.requestId))
        val receipt = consumer.state().commandRequests.single()
        assertEquals(WebChatConsumerCommandStatus.SUCCEEDED, receipt.status)
        assertEquals(url, WebChatConversationSharePolicy.resultUrl(receipt.detail))
        assertEquals("unsent fixture", snapshot.draft)
        snapshot = snapshot.copy(streaming = true)
        assertFalse(consumer.shareConversation("/c/fixture", true).accepted)
    }
}
