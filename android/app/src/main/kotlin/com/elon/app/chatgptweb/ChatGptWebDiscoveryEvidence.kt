package com.elon.app.chatgptweb

internal object ChatGptWebDiscoveryEvidence {
    data class Result(
        val status: String,
        val gap: String?,
    )

    fun resolve(
        caseId: String?,
        evidence: ChatGptWebVerificationEvidenceStore.Snapshot,
    ): Result {
        if (caseId == null) return Result("not_required", null)
        val record = evidence.records[caseId]
        return when {
            record?.current == true -> Result("device_observed", null)
            record == null -> Result(
                "not_recorded",
                "device_discovery_not_recorded_for_current_case_input",
            )
            else -> Result(
                "stale",
                "discovery_case_inputs_changed_since_device_observation",
            )
        }
    }
}
