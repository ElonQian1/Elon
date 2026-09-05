package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebPrivateProtocolEvidenceTest {
    private fun record() = JSONObject().put("id", 1).put("method", "POST")
        .put("path", "/backend-api/files/{id}").put("transport", "fetch").put("status", 201)
        .put("requestKind", "json").put("responseKind", "json")
        .put("requestState", "ready").put("responseState", "ready")
        .put("requestFields", JSONArray(listOf("$:object", "$.size:number")))
        .put("responseFields", JSONArray(listOf("$:object", "$.id:string")))

    private fun root(record: JSONObject = record()) = JSONObject()
        .put("schema", "elon.private_protocol_probe.v1").put("active", false).put("dropped", 0)
        .put("records", JSONArray().put(record))

    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail(
        "private_protocol_probe", value.toString(),
    )

    @Test fun structuralResultsRemainCompleteAndOrdinaryResultsRemainBounded() {
        val result = detail(root())
        assertTrue(result.length > 160)
        assertEquals(201, JSONObject(result).getJSONArray("records").getJSONObject(0).getInt("status"))
        assertEquals("x".repeat(160), ChatGptWebPrivateProtocolEvidence.detail("send_prompt", "x".repeat(500)))
        assertEquals("protocol_probe_unavailable", ChatGptWebPrivateProtocolEvidence.detail(
            "private_protocol_probe", "protocol_probe_unavailable"))
    }

    @Test fun rejectsRawValuesHeadersUnknownFieldsAndInvalidTypes() {
        val invalid = listOf(
            root().put("headers", JSONObject().put("Authorization", "secret")),
            root(record().put("raw", "private response")),
            root(record().put("path", "https://chatgpt.com/backend-api/files")),
            root(record().put("path", "/backend-api/files?token=secret")),
            root(record().put("responseFields", JSONArray(listOf("$.id:secret-value")))),
            root(record().put("status", "201")), root(record().put("status", 201.5)),
            root(record().put("id", 2)), root(record().put("method", "CONNECT")),
            root(record().put("responseState", "success")),
            root().put("active", "false"), root().put("dropped", -1),
        )
        invalid.forEach { assertEquals("invalid_protocol_evidence", detail(it)) }
    }

    @Test fun limitsOutputRecordsAndFieldsRatherThanTruncatingJson() {
        val tooMany = JSONArray()
        repeat(13) { tooMany.put(record().put("id", it + 1)) }
        assertEquals("invalid_protocol_evidence", detail(root().put("records", tooMany)))
        assertEquals("invalid_protocol_evidence", detail(root(record().put("requestFields",
            JSONArray(List(13) { "$.size:number" })))))
        assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail(
            "private_protocol_probe", " ".repeat(12001)))
        assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail(
            "private_protocol_probe", "{"))
    }

    @Test fun commandResultParserUsesTheStructuralValidator() {
        fun parse(detail: String) = ChatGptWebProtocol.parse(JSONObject()
            .put("type", "command_result").put("action", "private_protocol_probe")
            .put("requestId", "mcp_a9").put("ok", true).put("detail", detail).toString()
        ) as ChatGptWebEvent.CommandResult
        assertEquals("mcp_a9", parse(root().toString()).requestId)
        assertTrue(parse(root().toString()).detail.length > 160)
        assertEquals("invalid_protocol_evidence", parse("raw secret").detail)
    }
}
