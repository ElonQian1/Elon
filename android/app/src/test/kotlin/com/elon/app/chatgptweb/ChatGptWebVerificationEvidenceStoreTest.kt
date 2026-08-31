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
    fun acceptsKnownDiscoveryCasesWithoutTreatingUnknownCasesAsEvidence() {
        val discoveryCase = "reversible/composer_tool_discovery/deep_research"
        val hash = "d".repeat(64)
        val inputs = ChatGptWebVerificationEvidenceStore.currentInputs(
            org.json.JSONObject()
                .put(discoveryCase, hash)
                .put("reversible/composer_tool_discovery/unknown", hash)
                .toString(),
        )

        assertEquals(mapOf(discoveryCase to hash), inputs)
    }

    @Test
    fun parsesOnlyKnownPositiveContractRevisions() {
        val knownCase = ChatGptWebFeatureBaseline.evidenceCaseIds().first()
        val revisions = ChatGptWebVerificationEvidenceStore.currentContractRevisions(
            org.json.JSONObject()
                .put(knownCase, 2)
                .put("unknown/case", 3)
                .put("safe/session_recovery", 0)
                .toString(),
        )

        assertEquals(mapOf(knownCase to 2), revisions)
        assertTrue(ChatGptWebVerificationEvidenceStore.currentContractRevisions("bad").isEmpty())
    }

    @Test
    fun legacyEvidenceSurvivesImplementationDriftUntilItsContractChanges() {
        val caseId = "safe/session_recovery"
        val oldHash = "a".repeat(64)
        val raw = evidenceRecord("elon.chatgpt_web.verification_evidence_record.v1", caseId, oldHash)

        val sameContract = ChatGptWebVerificationEvidenceStore.parseRecord(
            raw,
            caseId,
            "b".repeat(64),
            currentContractRevision = 1,
        )!!
        val changedContract = ChatGptWebVerificationEvidenceStore.parseRecord(
            raw,
            caseId,
            "b".repeat(64),
            currentContractRevision = 2,
        )!!

        assertTrue(sameContract.current)
        assertFalse(sameContract.implementationCurrent)
        assertEquals(1, sameContract.contractRevision)
        assertFalse(changedContract.current)
    }

    @Test
    fun versionedEvidenceRequiresItsRecordedContractRevision() {
        val caseId = "safe/session_recovery"
        val hash = "c".repeat(64)
        val raw = evidenceRecord(
            "elon.chatgpt_web.verification_evidence_record.v2",
            caseId,
            hash,
            contractRevision = 3,
        )

        val record = ChatGptWebVerificationEvidenceStore.parseRecord(
            raw,
            caseId,
            hash,
            currentContractRevision = 3,
        )!!

        assertTrue(record.current)
        assertTrue(record.implementationCurrent)
        assertEquals(3, record.contractRevision)
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

        assertEquals("elon.chatgpt_web.verification_evidence.v2", json.getString("schema"))
        assertEquals("contract_revision", json.getString("current_basis"))
        assertEquals(1, json.getInt("registered_case_count"))
        assertEquals(1, json.getInt("current_case_count"))
        assertEquals(1, json.getInt("implementation_current_case_count"))
        assertEquals(0, json.getInt("implementation_drift_case_count"))
        assertEquals(caseId, json.getJSONArray("current_case_ids").getString(0))
        val serialized = json.toString()
        assertFalse(serialized.contains("cookie", ignoreCase = true))
        assertFalse(serialized.contains("conversation", ignoreCase = true))
        assertFalse(serialized.contains("account", ignoreCase = true))
    }

    private fun evidenceRecord(
        schema: String,
        caseId: String,
        inputSha256: String,
        contractRevision: Int? = null,
    ): String = org.json.JSONObject()
        .put("schema", schema)
        .put("case_id", caseId)
        .put("input_sha256", inputSha256)
        .put("contract_revision", contractRevision ?: org.json.JSONObject.NULL)
        .put("adapter_version", 1)
        .put("apk_version_name", "test")
        .put("apk_version_code", 1)
        .put("recorded_at_ms", 1L)
        .toString()
}
