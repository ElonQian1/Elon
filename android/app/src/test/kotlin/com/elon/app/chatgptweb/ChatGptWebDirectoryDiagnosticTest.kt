package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebDirectoryDiagnosticTest {
    private fun read() = JSONObject().put("family", "conversations").put("pages", 4)
        .put("resumedPages", 2).put("requests", 3).put("elapsedMs", 6000).put("lastRequestMs", 4000)
        .put("ok", false).put("complete", false).put("truncated", false).put("code", "directory_timeout")
    private fun value() = JSONObject().put("schema", ChatGptWebDirectoryDiagnostic.SCHEMA)
        .put("observed", true).put("durationMs", 6100).put("identityMs", 100).put("reads", JSONArray().put(read()))
    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString())

    @Test fun nativeProbeAdmitsTypedDirectoryTimings() {
        assertTrue("directory_refresh" in ChatGptWebPrivateProtocolEvidence.MODES)
        val input = value()
        assertEquals(input.toString(), detail(input))
        input.put("observed", false).put("reads", JSONArray())
        assertEquals(input.toString(), detail(input))
    }

    @Test fun diagnosticRejectsPrivateFieldsAndCoercion() {
        val invalid = mutableListOf(value().put("account", "private"), value().put("observed", "true"))
        for ((key, raw) in listOf("family" to "g-p-private", "code" to "directory_private_text",
            "pages" to 11, "requests" to -1, "elapsedMs" to "500", "complete" to 1,
            "lastRequestMs" to 60001, "cursor" to "private")) {
            invalid.add(value().put("reads", JSONArray().put(read().put(key, raw))))
        }
        invalid.add(value().put("reads", JSONArray().put(read()).put(read())))
        invalid.add(value().apply { remove("durationMs") })
        invalid.forEach { assertEquals("invalid_protocol_evidence", detail(it)) }
    }
}
