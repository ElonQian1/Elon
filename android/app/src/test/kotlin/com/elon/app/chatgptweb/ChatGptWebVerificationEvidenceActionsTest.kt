package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebVerificationEvidenceActionsTest {
    @Test
    fun requiresTheCurrentAdapterAndAnAuthenticatedSession() {
        val missingVersion = recordArgs().apply { remove("expected_adapter_version") }
        assertError(missingVersion, true, "missing_expected_adapter_version")

        val staleVersion = recordArgs().put(
            "expected_adapter_version",
            ChatGptWebPageAdapter.ADAPTER_VERSION - 1,
        )
        assertError(staleVersion, true, "adapter_version_mismatch")
        assertError(recordArgs(), false, "authentication_required")
    }

    @Test
    fun rejectsMalformedUnknownAndUnrecordedCases() {
        assertError(
            recordArgs().put("case_ids", JSONArray().put(123)),
            true,
            "invalid_case_id",
        )
        assertError(
            recordArgs("unknown/case"),
            true,
            "unknown_verification_case",
        )
        val result = ChatGptWebVerificationEvidenceActions.record(
            recordArgs(),
            authenticated = true,
            recorder = { ChatGptWebVerificationEvidenceStore.Snapshot.EMPTY },
        )
        assertEquals(
            "verification_evidence_not_current",
            (result as ChatGptWebVerificationEvidenceActions.Result.Error).code,
        )
    }

    @Test
    fun returnsOnlyCurrentStructuralEvidenceForKnownCases() {
        val caseId = "safe/session_recovery"
        val recorded = linkedSetOf<String>()
        val result = ChatGptWebVerificationEvidenceActions.record(
            recordArgs(caseId),
            authenticated = true,
            recorder = { caseIds ->
                recorded.addAll(caseIds)
                currentSnapshot(caseIds)
            },
        ) as ChatGptWebVerificationEvidenceActions.Result.Success

        assertEquals(setOf(caseId), recorded)
        assertTrue(result.response.getBoolean("control_ok"))
        assertEquals(caseId, result.response.getJSONArray("recorded_case_ids").getString(0))
        assertEquals(
            1,
            result.response.getJSONObject("verification_evidence").getInt("current_case_count"),
        )
    }

    @Test
    fun recordsKnownComposerToolDiscoveryCases() {
        val caseId = "reversible/composer_tool_discovery/study_mode"
        val recorded = linkedSetOf<String>()
        val result = ChatGptWebVerificationEvidenceActions.record(
            recordArgs(caseId),
            authenticated = true,
            recorder = { caseIds ->
                recorded.addAll(caseIds)
                currentSnapshot(caseIds)
            },
        ) as ChatGptWebVerificationEvidenceActions.Result.Success

        assertEquals(setOf(caseId), recorded)
        assertTrue(result.response.getBoolean("control_ok"))
    }

    private fun assertError(args: JSONObject, authenticated: Boolean, expected: String) {
        val result = ChatGptWebVerificationEvidenceActions.record(
            args,
            authenticated,
            recorder = { currentSnapshot(it) },
        )
        assertEquals(expected, (result as ChatGptWebVerificationEvidenceActions.Result.Error).code)
    }

    private fun recordArgs(caseId: String = "safe/session_recovery") = JSONObject()
        .put("expected_adapter_version", ChatGptWebPageAdapter.ADAPTER_VERSION)
        .put("case_ids", JSONArray().put(caseId))

    private fun currentSnapshot(caseIds: Set<String>): ChatGptWebVerificationEvidenceStore.Snapshot {
        val hash = "a".repeat(64)
        return ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = caseIds.associateWith { hash },
            records = caseIds.associateWith { caseId ->
                ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = hash,
                    current = true,
                    adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                )
            },
        )
    }
}
