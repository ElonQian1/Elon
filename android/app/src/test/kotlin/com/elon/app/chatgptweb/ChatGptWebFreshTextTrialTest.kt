package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebFreshTextTrialTest {
    private fun sample() = JSONObject().put("schema", ChatGptWebFreshTextTrial.SCHEMA).put("version", 4)
        .put("control", "state").put("armed", false).put("remaining_ms", 0).put("attempts", 1)
        .put("pending", false).put("phase", "completed").put("code", "")
        .put("dispatched", true).put("accepted", true).put("reconciled", true)

    @Test fun acceptsBoundedEvidenceThroughActualCommandReceiver() {
        val result = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", sample().toString())
        assertEquals("completed", JSONObject(result).getString("phase"))
        for (mode in listOf("start", "end", "state")) {
            assertTrue("fresh_text_trial_$mode" in ChatGptWebPrivateProtocolEvidence.MODES)
        }
    }

    @Test fun rejectsUnknownFieldsAndOutOfRangeCounters() {
        for (value in listOf(sample().put("content", "fixture"), sample().put("remaining_ms", 120001),
            sample().put("attempts", 33), sample().put("code", "/c/fixture"), sample().put("version", 5),
            sample().put("pending", "false"), sample().put("phase", "fixture"))) {
            assertEquals("invalid_protocol_evidence",
                ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString()))
        }
    }

    @Test fun endedPermissionCannotClaimTimeRemaining() {
        assertEquals("invalid_protocol_evidence", ChatGptWebPrivateProtocolEvidence.detail(
            "private_protocol_probe", sample().put("remaining_ms", 10).toString()))
    }
}
