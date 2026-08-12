package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebVerificationEvidenceActions {
    sealed interface Result {
        data class Success(val response: JSONObject) : Result
        data class Error(val code: String) : Result
    }

    fun record(
        args: JSONObject,
        authenticated: Boolean,
        recorder: (Set<String>) -> ChatGptWebVerificationEvidenceStore.Snapshot,
    ): Result {
        val values = args.optJSONArray("case_ids") ?: return Result.Error("missing_case_ids")
        if (values.length() > MAX_CASES) return Result.Error("too_many_case_ids")
        val expectedAdapterVersion = args.opt("expected_adapter_version") as? Number
            ?: return Result.Error("missing_expected_adapter_version")
        if (!isCurrentAdapter(expectedAdapterVersion)) {
            return Result.Error("adapter_version_mismatch")
        }
        if (!authenticated) return Result.Error("authentication_required")

        val caseIds = parseCaseIds(values) ?: return Result.Error("invalid_case_id")
        if (caseIds.isEmpty()) return Result.Error("missing_case_ids")
        if (!ChatGptWebFeatureBaseline.verificationCaseIds().containsAll(caseIds)) {
            return Result.Error("unknown_verification_case")
        }

        val evidence = recorder(caseIds)
        if (!evidence.currentCaseIds.containsAll(caseIds)) {
            return Result.Error("verification_evidence_not_current")
        }
        return Result.Success(
            JSONObject()
                .put("control_ok", true)
                .put("action", "chatgpt_record_verification_cases")
                .put("recorded_case_ids", JSONArray(caseIds.sorted()))
                .put("verification_evidence", evidence.toJson()),
        )
    }

    private fun parseCaseIds(values: JSONArray): Set<String>? {
        val caseIds = linkedSetOf<String>()
        for (index in 0 until values.length()) {
            val caseId = (values.opt(index) as? String)?.trim()
                ?.takeIf(String::isNotEmpty)
                ?: return null
            caseIds.add(caseId)
        }
        return caseIds
    }

    private fun isCurrentAdapter(value: Number): Boolean {
        val exact = value.toDouble()
        return exact.isFinite() &&
            exact % 1.0 == 0.0 &&
            exact == ChatGptWebPageAdapter.ADAPTER_VERSION.toDouble()
    }

    private const val MAX_CASES = 32
}
