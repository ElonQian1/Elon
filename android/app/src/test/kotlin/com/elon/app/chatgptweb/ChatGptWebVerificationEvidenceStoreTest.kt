package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebVerificationEvidenceStoreTest {
    @Test
    fun parsesOnlyKnownVerificationCasesWithValidBehaviorFingerprints() {
        val knownCase = ChatGptWebFeatureBaseline.verificationCaseIds().first()
        val validHash = "a".repeat(64)
        val raw = org.json.JSONObject()
            .put(knownCase, validHash)
            .put("unknown/case", "b".repeat(64))
            .put("safe/session_recovery", "invalid")
            .toString()

        val inputs = ChatGptWebVerificationEvidenceStore.currentInputs(raw)

        assertEquals(mapOf(knownCase to validHash), inputs)
        assertTrue(ChatGptWebVerificationEvidenceStore.currentInputs("not-json").isEmpty())
    }

    @Test
    fun snapshotReportsOnlyStructuralEvidenceAndCurrentCases() {
        val caseId = "safe/session_recovery"
        val currentHash = "c".repeat(64)
        val snapshot = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = mapOf(caseId to currentHash),
            records = mapOf(
                caseId to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = currentHash,
                    current = true,
                    adapterVersion = 79,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
            ),
        )

        val json = snapshot.toJson()

        assertEquals("elon.chatgpt_web.verification_evidence.v1", json.getString("schema"))
        assertEquals(1, json.getInt("registered_case_count"))
        assertEquals(1, json.getInt("current_case_count"))
        assertEquals(caseId, json.getJSONArray("current_case_ids").getString(0))
        val serialized = json.toString()
        assertFalse(serialized.contains("cookie", ignoreCase = true))
        assertFalse(serialized.contains("conversation", ignoreCase = true))
        assertFalse(serialized.contains("account", ignoreCase = true))
    }
}
