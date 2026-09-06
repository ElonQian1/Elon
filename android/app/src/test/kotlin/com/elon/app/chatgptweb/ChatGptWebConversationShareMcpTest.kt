package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatConsumerCommandStatus
import com.elon.app.WebChatConversationSharePolicy
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebConversationShareMcpTest {
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
