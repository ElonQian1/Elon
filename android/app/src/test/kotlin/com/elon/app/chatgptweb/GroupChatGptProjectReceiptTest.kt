package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class GroupChatGptProjectReceiptTest {
    private val scope = "a".repeat(64)
    private val project = "g-p-" + "b".repeat(32)
    private val conversation = "11111111-1111-4111-8111-111111111111"
    private fun ready() = JSONObject().put("ok", true).put("code", "project_ready")
        .put("accountScope", scope).put("projectId", project).put("memoryScope", "project_v2")
    private fun throughBridge(payload: JSONObject): ChatGptWebEvent.CommandResult =
        ChatGptWebProtocol.parse(JSONObject().put("type", "command_result")
            .put("adapterVersion", 437).put("documentToken", "doc_project_receipt")
            .put("action", "group_project_request").put("ok", payload.optBoolean("ok"))
            .put("requestId", "mcp_projecttest").put("detail", payload.toString()).toString()) as ChatGptWebEvent.CommandResult

    @Test fun projectAndConversationReceiptsSurviveTheRealBridgeParser() {
        for (payload in listOf(ready(), ready().put("conversationId", conversation))) {
            assertTrue(payload.toString().length > 160)
            val event = throughBridge(payload)
            val parsed = JSONObject(event.detail)
            assertTrue(parsed.getBoolean("ok"))
            assertEquals(project, parsed.getString("projectId"))
            assertEquals(scope, parsed.getString("accountScope"))
            assertEquals("project_v2", parsed.getString("memoryScope"))
            assertEquals(payload.optString("conversationId"), parsed.optString("conversationId"))
            assertEquals("mcp_projecttest", event.requestId)
        }
    }

    @Test fun identityAndFailureReceiptsRetainTheirTypedFields() {
        val identity = JSONObject().put("ok", true).put("code", "project_identity_ready").put("accountScope", scope)
        assertEquals(scope, JSONObject(throughBridge(identity).detail).getString("accountScope"))
        val error = JSONObject().put("ok", false).put("code", "project_identity_unavailable")
            .put("notSent", true).put("identityReason", "account")
        val result = JSONObject(throughBridge(error).detail)
        assertFalse(result.getBoolean("ok"))
        assertTrue(result.getBoolean("notSent"))
        assertEquals("account", result.getString("identityReason"))
    }

    @Test fun malformedAndUnexpectedFieldsNeverBecomeSuccessfulOrRetryableReceipts() {
        val invalid = listOf(ready().toString().take(160), "x".repeat(1025),
            ready().put("token", "private fixture").toString(),
            ready().put("accountScope", "wrong").toString(),
            ready().put("projectId", "../other").toString(),
            ready().put("memoryScope", "global").toString(),
            ready().put("conversationId", 123).toString(),
            JSONObject().put("ok", "true").put("code", "project_ready").toString(),
            JSONObject().put("ok", false).put("code", "project_create_unknown").put("notSent", true).toString(),
            JSONObject().put("ok", false).put("code", "project_unavailable").put("identityReason", "secret").toString())
        for (raw in invalid) {
            val result = JSONObject(GroupChatGptProjectReceipt.detail(raw))
            assertFalse(result.getBoolean("ok"))
            assertFalse(result.optBoolean("notSent"))
            assertEquals("project_receipt_invalid", result.getString("code"))
            assertEquals(2, result.length())
        }
    }

    @Test fun otherCommandDetailsKeepTheirExistingLimit() {
        assertEquals(160, ChatGptWebPrivateProtocolEvidence.detail("snapshot", "x".repeat(1024)).length)
    }
}
