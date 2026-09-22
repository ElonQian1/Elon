package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebScheduledTasksEvidenceTest {
    private fun source() = JSONObject().put("schema", "elon.scheduled_tasks_probe.v1")
        .put("catalogCount", 2).put("catalogComplete", true).put("cacheHit", true)
        .put("latestState", "update").put("updateCharacters", 120).put("lastRunFailed", JSONObject.NULL)

    @Test fun preservesOnlyStructuralEvidence() {
        assertTrue("scheduled_tasks" in ChatGptWebPrivateProtocolEvidence.MODES)
        val value = source()
        assertEquals(value.toString(), ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString()))
        val event = ChatGptWebProtocol.parse(JSONObject().put("type", "command_result")
            .put("action", "private_protocol_probe").put("requestId", "mcp_tasks1")
            .put("ok", true).put("detail", value.toString()).toString()) as ChatGptWebEvent.CommandResult
        assertEquals(value.toString(), event.detail)
    }

    @Test fun rejectsPrivateContentAndInvalidTypes() {
        for (value in listOf(source().put("title", "private"), source().put("updateCharacters", -1),
            source().put("catalogCount", 201), source().put("catalogComplete", "true"),
            source().put("latestState", "arbitrary body"), source().put("lastRunFailed", 1),
            source().apply { remove("lastRunFailed") })) {
            assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString()))
        }
    }
}
