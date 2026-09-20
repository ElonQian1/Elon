package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class GroupWebAiFailureInspectionTest {
    private class Harness {
        val reads = mutableListOf<String>()
        val timers = mutableListOf<() -> Unit>()
        val observations = mutableListOf<Map<String, Any?>>()
        var completed = 0
        val inspection = GroupWebAiFailureInspection(reads::add, { delay, task ->
            assertEquals(2_000L, delay); timers += task
        }, observations::add, { completed++ })
        fun event(detail: String, id: String? = reads.last(), action: String = "private_protocol_probe") =
            inspection.event(ChatGptWebEvent.CommandResult(action, true, detail, id))
        fun receipt(code: String = "context_changed") = JSONObject()
            .put("schema", ChatGptWebFreshTextTrial.SCHEMA).put("version", 5).put("control", "state")
            .put("armed", false).put("remaining_ms", 0).put("attempts", 1).put("pending", false)
            .put("phase", "rejected").put("code", code).put("dispatched", false).put("accepted", false)
            .put("reconciled", false).put("stream_events", 0).put("event_types", org.json.JSONArray())
            .put("history", "not_observed").toString()
    }

    @Test fun capturesOneFailureWithoutAWriteAndFinishesOnce() {
        val h = Harness(); repeat(3) { h.inspection.start() }
        assertEquals(1, h.reads.size)
        h.event(h.receipt()); h.event(h.receipt()); h.timers.forEach { it() }
        assertEquals(1, h.completed)
        assertEquals("context_changed", h.observations.single()["code"])
        assertEquals(false, h.observations.single()["dispatched"])
    }

    @Test fun missingReceiptHasBoundedCompletion() {
        val h = Harness(); h.inspection.start(); h.timers.single()()
        h.event(h.receipt())
        assertEquals(1, h.completed)
        assertEquals("receipt_timeout", h.observations.single()["code"])
    }

    @Test fun wrongCommandAndClosedInspectionCannotContinue() {
        val h = Harness(); h.inspection.start()
        h.event(h.receipt(), "mcp_other"); h.event(h.receipt(), action = "send_prompt")
        assertEquals(0, h.completed)
        h.inspection.close(); h.event(h.receipt()); h.timers.single()(); h.inspection.start()
        assertEquals(0, h.completed); assertTrue(h.observations.isEmpty()); assertEquals(1, h.reads.size)
    }

    @Test fun rejectsUntrustedPayloadAndNeverLogsArbitraryCodes() {
        val h = Harness(); h.inspection.start()
        h.event(h.receipt("private_user_content"))
        assertEquals("unknown", h.observations.single()["code"])
        assertFalse(h.observations.toString().contains("private_user_content"))
        val bad = Harness(); bad.inspection.start()
        bad.event(JSONObject(bad.receipt()).put("prompt", "private-content").toString())
        assertEquals("receipt_invalid", bad.observations.single()["code"])
        assertFalse(bad.observations.toString().contains("private-content"))
    }
}
