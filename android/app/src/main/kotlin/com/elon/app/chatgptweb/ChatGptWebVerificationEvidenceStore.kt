package com.elon.app.chatgptweb

import android.content.Context
import com.elon.app.BuildConfig
import org.json.JSONArray
import org.json.JSONObject

internal class ChatGptWebVerificationEvidenceStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val preferences = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)

    fun record(caseIds: Set<String>): Snapshot {
        val currentInputs = currentInputs()
        val currentContractRevisions = currentContractRevisions()
        val recordedAtMs = nowMs()
        val editor = preferences.edit()
        caseIds.sorted().forEach { caseId ->
            val inputSha256 = currentInputs[caseId] ?: return@forEach
            val contractRevision = currentContractRevisions[caseId] ?: return@forEach
            editor.putString(key(caseId), JSONObject()
                .put("schema", RECORD_SCHEMA)
                .put("case_id", caseId)
                .put("input_sha256", inputSha256)
                .put("contract_revision", contractRevision)
                .put("adapter_version", ChatGptWebPageAdapter.ADAPTER_VERSION)
                .put("apk_version_name", BuildConfig.VERSION_NAME)
                .put("apk_version_code", BuildConfig.VERSION_CODE)
                .put("recorded_at_ms", recordedAtMs)
                .toString())
        }
        check(editor.commit()) { "Unable to persist ChatGPT Web verification evidence." }
        return snapshot()
    }

    fun snapshot(): Snapshot {
        val inputs = currentInputs()
        val contractRevisions = currentContractRevisions()
        val records = inputs.mapValues { (caseId, inputSha256) ->
            val currentContractRevision = contractRevisions[caseId] ?: return@mapValues null
            parseRecord(
                preferences.getString(key(caseId), null),
                caseId,
                inputSha256,
                currentContractRevision,
            )
        }
        return Snapshot(inputs, records, contractRevisions)
    }

    data class Record(
        val caseId: String,
        val inputSha256: String,
        val current: Boolean,
        val adapterVersion: Int,
        val apkVersionName: String,
        val apkVersionCode: Int,
        val recordedAtMs: Long,
        val contractRevision: Int = DEFAULT_CONTRACT_REVISION,
        val implementationCurrent: Boolean = current,
    )

    data class Snapshot(
        val currentInputs: Map<String, String>,
        val records: Map<String, Record?>,
        val currentContractRevisions: Map<String, Int> = emptyMap(),
    ) {
        val currentCaseIds: Set<String>
            get() = records.filterValues { it?.current == true }.keys

        val implementationCurrentCaseIds: Set<String>
            get() = records.filterValues { it?.implementationCurrent == true }.keys

        val implementationDriftCaseIds: Set<String>
            get() = records.filterValues {
                it?.current == true && !it.implementationCurrent
            }.keys

        fun toJson(): JSONObject = JSONObject()
            .put("schema", SNAPSHOT_SCHEMA)
            .put("current_basis", "contract_revision")
            .put("registered_case_count", records.values.count { it != null })
            .put("current_case_count", currentCaseIds.size)
            .put("current_case_ids", JSONArray(currentCaseIds.sorted()))
            .put("implementation_current_case_count", implementationCurrentCaseIds.size)
            .put("implementation_drift_case_count", implementationDriftCaseIds.size)
            .put("implementation_drift_case_ids", JSONArray(implementationDriftCaseIds.sorted()))
            .put("cases", JSONArray().apply {
                currentInputs.forEach { (caseId, inputSha256) ->
                    val record = records[caseId]
                    put(JSONObject()
                        .put("case_id", caseId)
                        .put("current_input_sha256", inputSha256)
                        .put(
                            "current_contract_revision",
                            currentContractRevisions[caseId] ?: DEFAULT_CONTRACT_REVISION,
                        )
                        .put("registered", record != null)
                        .put("current", record?.current == true)
                        .put("implementation_current", record?.implementationCurrent == true)
                        .put("verified_input_sha256", record?.inputSha256 ?: JSONObject.NULL)
                        .put(
                            "verified_contract_revision",
                            record?.contractRevision ?: JSONObject.NULL,
                        )
                        .put("verified_adapter_version", record?.adapterVersion ?: JSONObject.NULL)
                        .put("verified_apk_version_name", record?.apkVersionName ?: JSONObject.NULL)
                        .put("verified_apk_version_code", record?.apkVersionCode ?: JSONObject.NULL)
                        .put("recorded_at_ms", record?.recordedAtMs ?: JSONObject.NULL))
                }
            })

        companion object {
            val EMPTY = Snapshot(emptyMap(), emptyMap())
        }
    }

    companion object {
        private const val PREFERENCES = "chatgpt_web_verification_evidence"
        private const val LEGACY_RECORD_SCHEMA = "elon.chatgpt_web.verification_evidence_record.v1"
        private const val RECORD_SCHEMA = "elon.chatgpt_web.verification_evidence_record.v2"
        private const val SNAPSHOT_SCHEMA = "elon.chatgpt_web.verification_evidence.v2"
        private const val DEFAULT_CONTRACT_REVISION = 1
        private val SHA256 = Regex("^[0-9a-f]{64}$")

        internal fun currentInputs(
            raw: String = BuildConfig.CHATGPT_WEB_VERIFICATION_CASE_INPUT_SHA256_JSON,
        ): Map<String, String> {
            val value = runCatching { JSONObject(raw) }.getOrNull() ?: return emptyMap()
            return value.keys().asSequence().mapNotNull { caseId ->
                val hash = value.optString(caseId)
                if (caseId in ChatGptWebFeatureBaseline.evidenceCaseIds() && SHA256.matches(hash)) {
                    caseId to hash
                } else {
                    null
                }
            }.sortedBy { it.first }.toMap(linkedMapOf())
        }

        internal fun currentContractRevisions(
            raw: String = BuildConfig.CHATGPT_WEB_VERIFICATION_CASE_CONTRACT_REVISION_JSON,
        ): Map<String, Int> {
            val value = runCatching { JSONObject(raw) }.getOrNull() ?: return emptyMap()
            return value.keys().asSequence().mapNotNull { caseId ->
                val revision = value.optInt(caseId, 0)
                if (caseId in ChatGptWebFeatureBaseline.evidenceCaseIds() && revision > 0) {
                    caseId to revision
                } else {
                    null
                }
            }.sortedBy { it.first }.toMap(linkedMapOf())
        }

        internal fun parseRecord(
            raw: String?,
            caseId: String,
            currentInput: String,
            currentContractRevision: Int,
        ): Record? {
            if (currentContractRevision <= 0) return null
            val value = raw?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return null
            val schema = value.optString("schema")
            if (schema != RECORD_SCHEMA && schema != LEGACY_RECORD_SCHEMA) return null
            if (value.optString("case_id") != caseId) return null
            val inputSha256 = value.optString("input_sha256")
            if (!SHA256.matches(inputSha256)) return null
            val contractRevision = if (schema == LEGACY_RECORD_SCHEMA) {
                DEFAULT_CONTRACT_REVISION
            } else {
                value.optInt("contract_revision", 0)
            }
            if (contractRevision <= 0) return null
            return Record(
                caseId = caseId,
                inputSha256 = inputSha256,
                current = contractRevision == currentContractRevision,
                adapterVersion = value.optInt("adapter_version"),
                apkVersionName = value.optString("apk_version_name"),
                apkVersionCode = value.optInt("apk_version_code"),
                recordedAtMs = value.optLong("recorded_at_ms"),
                contractRevision = contractRevision,
                implementationCurrent = inputSha256 == currentInput,
            )
        }

        private fun key(caseId: String): String = "case.$caseId"
    }
}
