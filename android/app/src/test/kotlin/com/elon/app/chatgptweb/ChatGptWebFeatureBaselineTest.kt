package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebFeatureBaselineTest {
    @Test
    fun recordsEveryFeatureWithAStableUniqueIdAndExplicitRemainingGap() {
        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )
        val features = baseline.getJSONArray("features")
        val ids = (0 until features.length()).map { index ->
            features.getJSONObject(index).getString("id")
        }

        assertEquals("elon.chatgpt_web.feature_baseline.v7", baseline.getString("schema"))
        assertEquals(ChatGptWebFeatureBaseline.VERSION, baseline.getInt("version"))
        assertEquals(
            ChatGptWebFeatureBaseline.DEVICE_VERIFICATION_ADAPTER_VERSION,
            baseline.getInt("device_verification_adapter_version"),
        )
        val deviceEvidenceCurrent = ChatGptWebFeatureBaseline.isDeviceVerificationCurrent()
        assertEquals(deviceEvidenceCurrent, baseline.getBoolean("device_verification_current"))
        assertTrue(BuildConfig.CHATGPT_WEB_INPUT_SHA256.matches(Regex("^[0-9a-f]{64}$")))
        assertEquals(
            BuildConfig.CHATGPT_WEB_VERIFIED_INPUT_SHA256,
            baseline.getString("device_verification_verified_input_sha256"),
        )
        assertEquals(
            BuildConfig.CHATGPT_WEB_INPUT_SHA256,
            baseline.getString("device_verification_input_sha256"),
        )
        val provenance = baseline.getJSONObject("device_verification_provenance")
        assertEquals("elon.chatgpt_web.device_evidence.v1", provenance.getString("schema"))
        assertEquals(1003, provenance.getInt("verified_apk_version_code"))
        assertEquals("1.1.993", provenance.getString("verified_apk_version_name"))
        assertEquals(
            "f97454749c2f23e480995faa51a7eab5badf5fd9",
            provenance.getString("verified_source_commit"),
        )
        assertEquals(ids.size, ids.toSet().size)
        assertEquals(ChatGptWebFeatureBaseline.ids(), ids.toSet())
        assertTrue(ids.containsAll(REQUIRED_FEATURE_IDS))
        assertEquals(0, baseline.getJSONObject("verification_evidence").getInt("current_case_count"))

        (0 until features.length()).forEach { index ->
            val feature = features.getJSONObject(index)
            val actions = feature.getJSONArray("mcp_actions")
            assertTrue(actions.length() > 0)
            (0 until actions.length()).forEach { actionIndex ->
                assertTrue(actions.getString(actionIndex) in AVAILABLE_MCP_ACTIONS)
            }
            if (feature.getString("code_status") != "partial") {
                assertTrue(feature.isNull("remaining_gap"))
            } else {
                assertFalse(feature.isNull("remaining_gap"))
            }
            if (feature.getString("code_status") == "partial") {
                assertFalse(feature.isNull("code_gap"))
            }
            if (feature.getString("verification_status") == "device_verified") {
                assertTrue(feature.isNull("verification_gap"))
                assertFalse(feature.isNull("verification_case"))
            } else {
                assertFalse(feature.isNull("verification_gap"))
                if (!feature.isNull("verification_case")) {
                    assertTrue(
                        feature.getString("verification_status") in
                            setOf("deferred", "user_action_required"),
                    )
                }
            }
        }
    }

    @Test
    fun invalidatesDeviceEvidenceWhenAdapterOrBehaviorInputsChange() {
        val verified = BuildConfig.CHATGPT_WEB_VERIFIED_INPUT_SHA256

        assertEquals(
            BuildConfig.CHATGPT_WEB_INPUT_SHA256 == verified,
            ChatGptWebFeatureBaseline.isDeviceVerificationCurrent(),
        )
        assertFalse(
            ChatGptWebFeatureBaseline.isDeviceVerificationCurrent(
                adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION + 1,
            ),
        )
        assertFalse(
            ChatGptWebFeatureBaseline.isDeviceVerificationCurrent(
                currentInputSha256 = "0".repeat(64),
            ),
        )
        assertFalse(
            ChatGptWebFeatureBaseline.isDeviceVerificationCurrent(
                currentInputSha256 = verified,
                verifiedInputSha256 = "not-a-sha256",
            ),
        )
    }

    @Test
    fun separatesStaticImplementationStateFromCurrentPageObservation() {
        val snapshot = ChatGptWebSnapshot(
            title = "Work",
            url = "https://chatgpt.com/",
            draft = "",
            messages = emptyList(),
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "5.6 Sol",
            attachments = emptyList(),
            dictationActive = false,
            capabilities = ChatGptWebCapabilities(
                setOf(
                    ChatGptWebCapabilityId.DICTATION,
                    ChatGptWebCapabilityId.CONVERSATION_LIST,
                ),
            ),
        )
        val manifest = ChatGptWebUiManifest(
            version = 3,
            pageKind = "home",
            title = "Work",
            compatibility = "healthy",
            controls = listOf(
                ChatGptWebUiControl(
                    id = "voice",
                    semantic = "voice_mode",
                    label = "Voice",
                    region = ChatGptWebUiRegion.COMPOSER,
                    role = "button",
                    enabled = true,
                    selected = false,
                ),
                ChatGptWebUiControl(
                    id = "control_project_tree",
                    semantic = "navigation",
                    label = "Project",
                    region = ChatGptWebUiRegion.CONTENT,
                    role = "treeitem",
                    enabled = true,
                    selected = false,
                    expanded = false,
                    expandable = true,
                ),
            ),
        )

        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = snapshot,
            manifest = manifest,
            mode = ChatGptWebModeController.Mode.NATIVE,
            composerOptions = listOf(
                ChatGptWebComposerOption(
                    "tools_study",
                    "学习",
                    false,
                    "menuitem",
                    ChatGptWebComposerOptionSemantics.STUDY,
                ),
            ),
        )

        assertTrue(feature(baseline, "official_authentication").getBoolean("current_page_observed"))
        assertTrue(feature(baseline, "native_chat_composer").getBoolean("current_page_observed"))
        assertTrue(feature(baseline, "conversation_history").getBoolean("current_page_observed"))
        val voice = feature(baseline, "realtime_voice")
        assertTrue(voice.getBoolean("current_page_observed"))
        assertEquals("fallback_only", voice.getString("implementation_status"))
        assertEquals("official_fallback", voice.getString("code_status"))
        assertEquals("user_action_required", voice.getString("verification_status"))
        assertTrue(feature(baseline, "disclosure_controls").getBoolean("current_page_observed"))
        assertFalse(feature(baseline, "projects").getBoolean("current_page_observed"))
        assertTrue(feature(baseline, "study_mode").getBoolean("current_page_observed"))
    }

    @Test
    fun reportsAConsistentSummaryAndRemainingFeatureList() {
        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.WEB,
        )
        val summary = baseline.getJSONObject("summary")
        val codeSummary = baseline.getJSONObject("code_summary")
        val verificationSummary = baseline.getJSONObject("verification_summary")

        assertEquals(
            baseline.getInt("feature_count"),
            summary.getInt("complete") +
                summary.getInt("partial") +
                summary.getInt("fallback_only"),
        )
        assertEquals(
            summary.getInt("remaining"),
            baseline.getJSONArray("remaining_feature_ids").length(),
        )
        assertTrue(summary.getInt("complete") > 0)
        assertEquals(0, summary.getInt("partial"))
        assertEquals(1, summary.getInt("fallback_only"))
        assertEquals(0, summary.getInt("remaining"))
        assertEquals(44, codeSummary.getInt("implemented"))
        assertEquals(0, codeSummary.getInt("partial"))
        assertEquals(1, codeSummary.getInt("official_fallback"))
        assertEquals(0, codeSummary.getInt("remaining"))
        assertEquals(0, verificationSummary.getInt("offline_verified"))
        assertEquals(0, verificationSummary.getInt("device_verified"))
        assertEquals(0, verificationSummary.getInt("verified"))
        assertEquals(35, verificationSummary.getInt("pending"))
        assertEquals(10, verificationSummary.getInt("user_action_required"))
        assertEquals(35, verificationSummary.getInt("deferred"))
        assertEquals(0, verificationSummary.getInt("failed"))
        assertEquals(45, verificationSummary.getInt("remaining"))
        assertEquals(0, baseline.getJSONArray("remaining_code_feature_ids").length())
        assertEquals("complete", feature(baseline, "model_selection").getString("implementation_status"))
        assertEquals("implemented", feature(baseline, "model_selection").getString("code_status"))
        assertEquals(
            "deferred",
            feature(baseline, "model_selection").getString("verification_status"),
        )
        assertEquals(
            "deferred",
            feature(baseline, "composer_tools").getString("verification_status"),
        )
        assertFalse(feature(baseline, "model_selection").isNull("verification_gap"))
        assertEquals(
            "deferred",
            feature(baseline, "native_chat_composer").getString("verification_status"),
        )
        assertEquals(
            "deferred",
            feature(baseline, "streaming_and_stop").getString("verification_status"),
        )
        assertEquals(
            "deferred",
            feature(baseline, "conversation_create_and_switch").getString("verification_status"),
        )
        assertEquals("complete", feature(baseline, "message_copy").getString("implementation_status"))
        assertEquals("implemented", feature(baseline, "message_copy").getString("code_status"))
        assertEquals(
            "deferred",
            feature(baseline, "message_copy").getString("verification_status"),
        )
        assertEquals(
            "reversible/copy_receipt_without_content_readback",
            feature(baseline, "message_copy").getString("verification_case"),
        )
        assertEquals("complete", feature(baseline, "disclosure_controls").getString("implementation_status"))
        assertEquals("implemented", feature(baseline, "disclosure_controls").getString("code_status"))
        assertEquals(
            "deferred",
            feature(baseline, "disclosure_controls").getString("verification_status"),
        )
        assertFalse(feature(baseline, "disclosure_controls").isNull("verification_gap"))
        assertEquals(
            "deferred",
            feature(baseline, "session_continuity_and_recovery").getString("verification_status"),
        )
        assertEquals(
            "deferred",
            feature(baseline, "session_long_running_stability").getString("verification_status"),
        )
        assertFalse(feature(baseline, "session_long_running_stability").isNull("verification_gap"))
        assertEquals(
            "complete",
            feature(baseline, "session_long_running_stability").getString("implementation_status"),
        )
        listOf(
            "projects",
            "tasks",
            "library",
            "gpts",
            "apps",
            "work",
            "settings",
            "adaptive_form_controls",
        ).forEach { id ->
            assertEquals(
                "deferred",
                feature(baseline, id).getString("verification_status"),
            )
        }
        listOf("health", "finances").forEach { id ->
            assertEquals(
                "user_action_required",
                feature(baseline, id).getString("verification_status"),
            )
        }
        listOf(
            "deep_research",
            "image_generation",
            "canvas",
            "study_mode",
        ).forEach { id ->
            assertEquals("deferred", feature(baseline, id).getString("verification_status"))
            assertTrue(feature(baseline, id).getString("verification_case").contains("tool_execution"))
        }
        assertEquals(
            "user_action_required",
            feature(baseline, "agent_mode").getString("verification_status"),
        )
        assertEquals(
            "supervised/composer_tool_execution/agent_mode",
            feature(baseline, "agent_mode").getString("verification_case"),
        )
    }

    @Test
    fun promotesOnlyTheFeaturePageWhoseCurrentDeviceCaseWasRecorded() {
        val caseId = "safe/feature_page/projects"
        val currentHash = "e".repeat(64)
        val evidence = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = linkedMapOf(caseId to currentHash),
            records = linkedMapOf(
                caseId to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = currentHash,
                    current = true,
                    adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
            ),
        )

        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
            verificationEvidence = evidence,
        )

        assertEquals("device_verified", feature(baseline, "projects").getString("verification_status"))
        assertEquals("deferred", feature(baseline, "tasks").getString("verification_status"))
        assertEquals("deferred", feature(baseline, "gpts").getString("verification_status"))
    }

    @Test
    fun resolvesDeviceEvidencePerVerificationCaseWithoutInvalidatingUnrelatedCases() {
        val currentHash = "a".repeat(64)
        val staleHash = "b".repeat(64)
        val evidence = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = linkedMapOf(
                "reversible/composer_controls" to currentHash,
                "safe/session_recovery" to "c".repeat(64),
            ),
            records = linkedMapOf(
                "reversible/composer_controls" to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = "reversible/composer_controls",
                    inputSha256 = currentHash,
                    current = true,
                    adapterVersion = 79,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
                "safe/session_recovery" to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = "safe/session_recovery",
                    inputSha256 = staleHash,
                    current = false,
                    adapterVersion = 78,
                    apkVersionName = "old",
                    apkVersionCode = 0,
                    recordedAtMs = 100L,
                ),
            ),
        )

        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
            verificationEvidence = evidence,
        )

        assertEquals("device_verified", feature(baseline, "web_search").getString("verification_status"))
        assertTrue(feature(baseline, "web_search").isNull("verification_gap"))
        assertEquals("deferred", feature(baseline, "session_continuity_and_recovery").getString("verification_status"))
        assertEquals(
            "verification_case_inputs_changed_since_device_acceptance",
            feature(baseline, "session_continuity_and_recovery").getString("verification_gap"),
        )
        assertEquals("deferred", feature(baseline, "model_selection").getString("verification_status"))
        assertEquals(1, baseline.getJSONObject("verification_evidence").getInt("current_case_count"))
    }

    @Test
    fun promotesRegenerateOnlyAfterItsCurrentDeviceCaseIsRecorded() {
        val currentHash = "d".repeat(64)
        val caseId = "reversible/regenerate_response"
        val baselineWithoutEvidence = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )
        assertEquals(
            "complete",
            feature(baselineWithoutEvidence, "message_regenerate")
                .getString("implementation_status"),
        )
        assertEquals(
            "deferred",
            feature(baselineWithoutEvidence, "message_regenerate")
                .getString("verification_status"),
        )

        val evidence = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = linkedMapOf(caseId to currentHash),
            records = linkedMapOf(
                caseId to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = currentHash,
                    current = true,
                    adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
            ),
        )
        val verifiedBaseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
            verificationEvidence = evidence,
        )

        val regenerate = feature(verifiedBaseline, "message_regenerate")
        assertEquals(caseId, regenerate.getString("verification_case"))
        assertEquals("device_verified", regenerate.getString("verification_status"))
        assertTrue(regenerate.isNull("verification_gap"))
    }

    @Test
    fun supervisedCasesRemainUserDrivenUntilCurrentEvidenceIsRecorded() {
        val caseId = "supervised/attachment_lifecycle"
        val withoutEvidence = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )
        val pending = feature(withoutEvidence, "attachment_lifecycle")
        assertEquals(caseId, pending.getString("verification_case"))
        assertEquals("user_action_required", pending.getString("verification_status"))
        assertEquals(
            "supervised/dictation_transcription",
            feature(withoutEvidence, "dictation").getString("verification_case"),
        )
        assertEquals(
            "supervised/realtime_voice_round_trip",
            feature(withoutEvidence, "realtime_voice").getString("verification_case"),
        )

        val hash = "a".repeat(64)
        val evidence = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = linkedMapOf(caseId to hash),
            records = linkedMapOf(
                caseId to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = hash,
                    current = true,
                    adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
            ),
        )
        val verified = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
            verificationEvidence = evidence,
        )
        assertEquals(
            "device_verified",
            feature(verified, "attachment_lifecycle").getString("verification_status"),
        )
    }

    @Test
    fun reportsComposerToolDiscoveryWithoutPromotingEndToEndVerification() {
        val caseId = "reversible/composer_tool_discovery/deep_research"
        val currentHash = "f".repeat(64)
        val evidence = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = linkedMapOf(caseId to currentHash),
            records = linkedMapOf(
                caseId to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = currentHash,
                    current = true,
                    adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
            ),
        )

        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
            verificationEvidence = evidence,
        )

        val deepResearch = feature(baseline, "deep_research")
        assertEquals("device_observed", deepResearch.getString("discovery_status"))
        assertTrue(deepResearch.isNull("discovery_gap"))
        assertEquals("deferred", deepResearch.getString("verification_status"))
        val image = feature(baseline, "image_generation")
        assertEquals("not_recorded", image.getString("discovery_status"))
        assertFalse(image.isNull("discovery_gap"))
        val summary = baseline.getJSONObject("discovery_summary")
        assertEquals(5, summary.getInt("required"))
        assertEquals(1, summary.getInt("device_observed"))
        assertEquals(4, summary.getInt("remaining"))
    }

    @Test
    fun promotesOnlyTheComposerToolWhoseExecutionCaseWasRecorded() {
        val caseId = "reversible/composer_tool_execution/deep_research"
        val currentHash = "9".repeat(64)
        val evidence = ChatGptWebVerificationEvidenceStore.Snapshot(
            currentInputs = linkedMapOf(caseId to currentHash),
            records = linkedMapOf(
                caseId to ChatGptWebVerificationEvidenceStore.Record(
                    caseId = caseId,
                    inputSha256 = currentHash,
                    current = true,
                    adapterVersion = ChatGptWebPageAdapter.ADAPTER_VERSION,
                    apkVersionName = "test",
                    apkVersionCode = 1,
                    recordedAtMs = 123L,
                ),
            ),
        )

        val baseline = ChatGptWebFeatureBaseline.describe(
            snapshot = null,
            manifest = null,
            mode = ChatGptWebModeController.Mode.NATIVE,
            verificationEvidence = evidence,
        )

        assertEquals("device_verified", feature(baseline, "deep_research").getString("verification_status"))
        assertEquals("deferred", feature(baseline, "image_generation").getString("verification_status"))
        assertEquals("user_action_required", feature(baseline, "agent_mode").getString("verification_status"))
    }

    private fun feature(baseline: org.json.JSONObject, id: String): org.json.JSONObject {
        val features = baseline.getJSONArray("features")
        return (0 until features.length())
            .map(features::getJSONObject)
            .first { it.getString("id") == id }
    }

    private companion object {
        val REQUIRED_FEATURE_IDS = setOf(
            "official_authentication",
            "anonymous_chat_access",
            "official_fullscreen_fallback",
            "native_chat_composer",
            "streaming_and_stop",
            "conversation_context_paging",
            "conversation_history",
            "temporary_chat",
            "model_selection",
            "attachment_lifecycle",
            "composer_tools",
            "web_search",
            "deep_research",
            "image_generation",
            "canvas",
            "study_mode",
            "agent_mode",
            "dictation",
            "realtime_voice",
            "message_action_context",
            "message_actions",
            "account_mutations",
            "conversation_mutations",
            "projects",
            "tasks",
            "library",
            "gpts",
            "apps",
            "settings",
            "health",
            "finances",
            "work",
            "adaptive_form_controls",
            "disclosure_controls",
            "official_change_detection",
            "stable_mcp_and_adb_controls",
            "session_continuity_and_recovery",
            "session_long_running_stability",
        )
        val AVAILABLE_MCP_ACTIONS = setOf(
            "state",
            "set_input_text",
            "send_input",
            "chatgpt_invoke_control",
            "chatgpt_set_control_text",
            "chatgpt_set_control_selected",
            "chatgpt_select_control_choice",
            "chatgpt_set_control_slider",
            "chatgpt_set_control_expanded",
            "chatgpt_new_conversation",
            "chatgpt_stop_generation",
            "chatgpt_copy_last_response",
            "chatgpt_regenerate_response",
            "chatgpt_start_dictation",
            "chatgpt_start_realtime_voice",
            "chatgpt_remove_attachment",
            "chatgpt_list_conversations",
            "chatgpt_list_composer_options",
            "chatgpt_select_composer_option",
            "chatgpt_select_feature",
            "chatgpt_get_context",
            "chatgpt_find_controls",
            "chatgpt_get_conversations",
            "chatgpt_get_navigation",
            "chatgpt_get_capability_matrix",
            "chatgpt_record_verification_cases",
            "chatgpt_open_conversation",
            "chatgpt_select_view",
        )
    }
}
