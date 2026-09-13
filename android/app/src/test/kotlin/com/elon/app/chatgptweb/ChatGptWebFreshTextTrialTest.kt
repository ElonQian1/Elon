package com.elon.app.chatgptweb

import org.json.JSONObject
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebFreshTextTrialTest {
    private fun sample() = JSONObject().put("schema", ChatGptWebFreshTextTrial.SCHEMA).put("version", 7)
        .put("control", "state").put("armed", false).put("remaining_ms", 0).put("attempts", 1)
        .put("pending", false).put("phase", "completed").put("code", "")
        .put("dispatched", true).put("accepted", true).put("reconciled", true)
        .put("stream_events", 3).put("event_types", org.json.JSONArray(listOf("delta_encoding", "message")))
        .put("history", "reconciled").put("parent_role", "assistant").put("operation", "send")

    @Test fun acceptsSharedFixturesVerifiedAgainstTheActualJavascriptTransaction() {
        val fixture = JSONObject(File("../../scripts/fixtures/chatgpt-fresh-trial-wire.json").readText())
        val cases = fixture.getJSONObject("cases")
        assertEquals(7, cases.length())
        for (name in cases.keys()) {
            val raw = JSONObject(fixture.getJSONObject("common").toString())
            val changes = cases.getJSONObject(name)
            for (key in changes.keys()) raw.put(key, changes.get(key))
            val output = JSONObject(ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", raw.toString()))
            assertTrue(name, raw.similar(output))
        }
    }

    @Test fun acceptsBoundedEvidenceThroughActualCommandReceiver() {
        val result = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", sample().toString())
        assertEquals("completed", JSONObject(result).getString("phase"))
        for (mode in listOf("start", "end", "state")) {
            assertTrue("fresh_text_trial_$mode" in ChatGptWebPrivateProtocolEvidence.MODES)
        }
    }

    @Test fun rejectsUnknownFieldsAndOutOfRangeCounters() {
        for (value in listOf(sample().put("content", "fixture"), sample().put("remaining_ms", 120001),
            sample().put("attempts", 65536), sample().put("code", "/c/fixture"), sample().put("version", 8),
            sample().put("parent_role", "private_fixture"),
            sample().put("pending", "false"), sample().put("phase", "fixture"))) {
            assertEquals("invalid_protocol_evidence",
                ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString()))
        }
    }

    @Test fun rejectsPrivateOrUnboundedStreamDiagnostics() {
        for (value in listOf(sample().put("stream_events", 65536), sample().put("history", "private_fixture"),
            sample().put("event_types", org.json.JSONArray(listOf("private_fixture"))),
            sample().put("event_types", org.json.JSONArray(listOf("message", "message"))))) {
            assertEquals("invalid_protocol_evidence",
                ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString()))
        }
    }

    @Test fun endedPermissionCannotClaimTimeRemaining() {
        assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail(
            "private_protocol_probe", sample().put("remaining_ms", 10).toString()))
    }

    @Test fun acceptsMoreThanTrialBudgetWithoutUnboundedCounters() {
        for (count in listOf(33, 1000, 65535)) {
            val result = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", sample().put("attempts", count).toString())
            assertEquals(count, JSONObject(result).getInt("attempts"))
        }
    }

    @Test fun retainsVersionFiveReceiptsWhileAnOlderWriterIsStillActive() {
        val old = sample().put("version", 5).put("attempts", 32)
        old.remove("parent_role")
        old.remove("operation")
        assertEquals(32, JSONObject(ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", old.toString())).getInt("attempts"))
        assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail(
            "private_protocol_probe", old.put("attempts", 33).toString()))
    }

    @Test fun acceptsBothShippedVersionSixShapesButRejectsUnreviewedOperations() {
        for (operation in listOf(null, "", "send", "regenerate")) {
            val old = sample().put("version", 6)
            if (operation == null) old.remove("operation") else old.put("operation", operation)
            val output = JSONObject(ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", old.toString()))
            assertTrue(old.similar(output))
        }
        for (version in listOf(6, 7)) for (operation in listOf("private_fixture", true, 1, JSONObject.NULL)) {
            assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe",
                sample().put("version", version).put("operation", operation).toString()))
        }
        for (value in listOf(sample().apply { remove("operation") }, sample().put("version", "7"),
            sample().put("version", 5).apply { remove("parent_role") })) {
            assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString()))
        }
    }
}
