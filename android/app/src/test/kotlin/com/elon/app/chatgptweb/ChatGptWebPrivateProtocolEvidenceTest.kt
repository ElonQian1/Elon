package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebPrivateProtocolEvidenceTest {
    @Test fun downloadSourceAdmitsOnlyClosedStructuralEvidence() {
        assertTrue("file_download_source" in ChatGptWebPrivateProtocolEvidence.MODES)
        fun source() = JSONObject().put("schema", "elon.download_source.v1").put("observed", true)
            .put("origin", "same_origin").put("path", "/api/library/files/{id}/download")
            .put("relative", false).put("whitespace", false).put("credentials", false)
            .put("port", false).put("fragment", false)
        assertEquals(source().toString(), detail(source()))
        val project = source().put("path", "/api/library/files/{id}/project-content")
        assertEquals(project.toString(), detail(project))
        for (value in listOf(source().put("url", "secret"), source().put("path", "/api/files/private"),
            source().put("path", "/api/content?token=secret"), source().put("origin", "private.test"),
            source().put("observed", "true"), source().apply { remove("port") })) {
            assertEquals("invalid_protocol_evidence", detail(value))
        }
    }

    @Test fun pairedDownloadSourceHasOnlyBoundedBindingEvidence() {
        fun source() = JSONObject().put("schema", "elon.download_source.v2").put("observed", true)
            .put("origin", "same_origin").put("path", "/api/library/files/{id}/project-content")
            .put("relative", true).put("whitespace", false).put("credentials", false)
            .put("port", false).put("fragment", false).put("binding", "matched").put("query_count", 2)
        for (binding in listOf("not_applicable", "scope_missing", "scope_invalid", "library_mismatch",
            "file_missing", "file_duplicate", "file_mismatch", "matched")) {
            val value = source().put("binding", binding)
            assertEquals(value.toString(), detail(value))
        }
        for (value in listOf(source().put("binding", "private"), source().put("query_count", "2"),
            source().put("query_count", -1), source().put("query_count", 1000),
            source().put("file_id", "private"), source().apply { remove("binding") })) {
            assertEquals("invalid_protocol_evidence", detail(value))
        }
    }

    @Test fun modelContextAllowsKnownStagesAndRejectsUnboundedValues() {
        assertTrue("model_runtime_context" in ChatGptWebPrivateProtocolEvidence.MODES)
        for (code in listOf("not_observed", "identity_unavailable", "picker_missing", "owner_unavailable",
            "runtime_not_observed", "runtime_unknown", "catalog_unavailable", "ready")) {
            val raw = "model_runtime_context:$code"
            assertEquals(raw, ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", raw))
        }
        for (raw in listOf("model_runtime_context:secret", "model_runtime_context:ready:secret")) {
            assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", raw))
        }
    }

    private fun stopOwner() = JSONObject().put("schema", "elon.stop_runtime_owner.v1")
        .put("cached", true).put("request", "missing").put("tree", true)
        .put("generation", false).put("mode", "streaming")

    @Test fun stopOwnerModeAndTypedShapeReachTheNativeLedger() {
        assertTrue("stop_runtime_owner" in ChatGptWebPrivateProtocolEvidence.MODES)
        for (request in listOf("missing", "valid", "invalid")) {
            for (mode in listOf("unknown", "idle", "streaming", "unread", "voice")) {
                val value = stopOwner().put("request", request).put("mode", mode)
                val result = JSONObject(detail(value))
                assertEquals(value.toString(), result.toString())
            }
        }
        val event = ChatGptWebProtocol.parse(JSONObject().put("type", "command_result")
            .put("action", "private_protocol_probe").put("requestId", "mcp_a9")
            .put("ok", true).put("detail", stopOwner().toString()).toString()) as ChatGptWebEvent.CommandResult
        assertEquals(stopOwner().toString(), event.detail)
        assertEquals("mcp_a9", event.requestId)
    }

    @Test fun stopOwnerRejectsCoercionMissingFieldsAndPrivateValues() {
        val invalid = mutableListOf(
            stopOwner().put("requestId", "secret"), stopOwner().put("text", "private"),
            stopOwner().put("mode", "secret"), stopOwner().put("request", "secret"),
            stopOwner().put("mode", JSONObject()), stopOwner().put("schema", "unknown"),
        )
        for (key in listOf("cached", "tree", "generation")) {
            for (value in listOf("true", 1, JSONObject.NULL)) invalid.add(stopOwner().put(key, value))
            invalid.add(stopOwner().apply { remove(key) })
        }
        invalid.forEach { assertEquals("invalid_protocol_evidence", detail(it)) }
    }

    @Test fun stopContextAllowsOnlyKnownStagesWithoutRequestOrConversationData() {
        assertTrue("stop_runtime_context" in ChatGptWebPrivateProtocolEvidence.MODES)
        for (code in listOf("not_observed", "composer_unavailable", "invoked", "stop_observed", "timeout")) {
            val value = "stop_runtime_context:$code"
            assertEquals(value, ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value))
        }
        for (raw in listOf("stop_runtime_context:secret", "stop_runtime_context:invoked:request-id")) {
            assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", raw))
        }
    }

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

    @Test fun toolContextAllowsOnlyBoundedDiagnosticCodes() {
        assertTrue("composer_tool_context" in ChatGptWebPrivateProtocolEvidence.MODES)
        for (code in listOf("not_observed", "ready", "cache_unavailable", "model_unavailable")) {
            val value = "composer_tool_context:$code"
            assertEquals(value, ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value))
        }
        for (raw in listOf("composer_tool_context:private data", "composer_tool_context:ready:extra",
            "composer_tool_context:ready\n")) {
            assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", raw))
        }
    }
}
