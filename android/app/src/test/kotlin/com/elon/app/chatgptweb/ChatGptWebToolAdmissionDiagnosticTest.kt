package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebToolAdmissionDiagnosticTest {
    private fun row() = JSONObject().put("tool", "study").put("raw", 1).put("menu", 0).put("reason", "menu_filtered")
    private fun payload() = JSONObject().put("schema", ChatGptWebToolAdmissionDiagnostic.SCHEMA)
        .put("observed", true).put("items", JSONArray().put(row()))
    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString())

    @Test fun validObservationSurvivesTheRealMcpReceiptBoundary() {
        assertTrue("composer_tool_admission" in ChatGptWebPrivateProtocolEvidence.MODES)
        assertEquals(payload().toString(), detail(payload()))
        val empty = payload().put("observed", false).put("items", JSONArray())
        assertEquals(empty.toString(), detail(empty))
    }

    @Test fun rejectsUnboundedDataAndPrivateFields() {
        for (invalid in listOf(payload().put("model", "fixture"), payload().put("observed", false),
            payload().put("items", JSONArray().put(row()).put(row())),
            payload().put("items", JSONArray().put(row().put("name", "fixture"))),
            payload().put("items", JSONArray().put(row().put("tool", "unknown"))),
            payload().put("items", JSONArray().put(row().put("reason", "unknown"))))) {
            assertEquals("invalid_protocol_evidence", detail(invalid))
        }
    }

    @Test fun countsMustBeBoundedIntegers() {
        for (count in listOf<Any>(-1, 3, 1.5, "1", JSONObject.NULL)) {
            for (key in listOf("raw", "menu")) {
                val value = payload().put("items", JSONArray().put(row().put(key, count)))
                assertEquals("invalid_protocol_evidence", detail(value))
            }
        }
    }
}
