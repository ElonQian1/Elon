package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebPrivateProtocolMcpTest {
    @Test fun explicitProbeUsesTheCommandLedgerWithoutSendingOrNavigating() {
        val state = ChatGptWebObservedState()
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_test"))
        val calls = mutableListOf<Pair<String, String>>()
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onOpenConversation = { fail("probe must not navigate") },
        ) {
            override fun privateProtocolProbe(mode: String, requestId: String) {
                calls.add(mode to requestId)
            }
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { null }, uiManifest = { null }, observedState = state::snapshot,
            beginCommand = state::beginCommand, bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebPresentationMode.NATIVE }, inputText = { "unsent draft" },
            setInputText = { fail("probe must not change draft") }, commands = commands,
            refresh = { fail("probe must not reload") }, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        assertTrue(ChatGptWebMcpActionCatalog.availableActions.contains("chatgpt_private_protocol_probe"))
        fun call(mode: String) = actions.control(JSONObject().put("action", "chatgpt_private_protocol_probe")
            .put("mode", mode))
        call("start")
        assertEquals("start", calls.single().first)
        val request = state.snapshot().commandRequests.single()
        assertEquals("private_protocol_probe", request.expectedAction)
        assertEquals(request.id, calls.single().second)
        val invalid = call("upload")
        assertFalse(invalid.getBoolean("control_ok"))
        assertEquals("invalid_probe_mode", invalid.getString("error"))
        assertEquals(1, calls.size)
        val detail = JSONObject().put("schema", "elon.private_protocol_probe.v1")
            .put("active", true).put("dropped", 0).put("records", org.json.JSONArray()).toString()
        state.accept(ChatGptWebEvent.CommandResult("private_protocol_probe", true, detail, request.id))
        val receipt = ChatGptWebCommandReceipts.requestsJson(state.snapshot()).getJSONObject(0)
        assertEquals(detail, receipt.getJSONObject("result").getString("detail"))
        state.updateDocument(WebBridgeDocumentSession.Snapshot(2, 0, "doc_next"))
        assertFalse(call("start").getBoolean("control_ok"))
        assertEquals(1, calls.size)
    }
}
