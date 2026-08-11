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

        assertEquals("elon.chatgpt_web.feature_baseline.v4", baseline.getString("schema"))
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
        assertEquals(968, provenance.getInt("verified_apk_version_code"))
        assertEquals("1.1.958", provenance.getString("verified_apk_version_name"))
        assertEquals(
            "1fc5c5fe75439a5a7161805baad5af4608dcd6c7",
            provenance.getString("verified_source_commit"),
        )
        assertEquals(ids.size, ids.toSet().size)
        assertEquals(ChatGptWebFeatureBaseline.ids(), ids.toSet())
        assertTrue(ids.containsAll(REQUIRED_FEATURE_IDS))

        (0 until features.length()).forEach { index ->
            val feature = features.getJSONObject(index)
            val actions = feature.getJSONArray("mcp_actions")
            assertTrue(actions.length() > 0)
            (0 until actions.length()).forEach { actionIndex ->
                assertTrue(actions.getString(actionIndex) in AVAILABLE_MCP_ACTIONS)
            }
            if (feature.getString("implementation_status") == "complete") {
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
                    assertEquals("deferred", feature.getString("verification_status"))
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
        val deviceEvidenceCurrent = baseline.getBoolean("device_verification_current")

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
        assertTrue(summary.getInt("partial") > 0)
        assertTrue(summary.getInt("fallback_only") > 0)
        assertEquals(32, codeSummary.getInt("implemented"))
        assertEquals(0, codeSummary.getInt("partial"))
        assertEquals(1, codeSummary.getInt("official_fallback"))
        assertEquals(0, codeSummary.getInt("remaining"))
        assertEquals(14, verificationSummary.getInt("offline_verified"))
        assertEquals(if (deviceEvidenceCurrent) 12 else 0, verificationSummary.getInt("device_verified"))
        assertEquals(if (deviceEvidenceCurrent) 12 else 0, verificationSummary.getInt("verified"))
        assertEquals(if (deviceEvidenceCurrent) 14 else 26, verificationSummary.getInt("pending"))
        assertEquals(7, verificationSummary.getInt("user_action_required"))
        assertEquals(if (deviceEvidenceCurrent) 0 else 12, verificationSummary.getInt("deferred"))
        assertEquals(0, verificationSummary.getInt("failed"))
        assertEquals(if (deviceEvidenceCurrent) 21 else 33, verificationSummary.getInt("remaining"))
        assertEquals(0, baseline.getJSONArray("remaining_code_feature_ids").length())
        assertEquals("complete", feature(baseline, "model_selection").getString("implementation_status"))
        assertEquals("implemented", feature(baseline, "model_selection").getString("code_status"))
        assertEquals(
            if (deviceEvidenceCurrent) "device_verified" else "deferred",
            feature(baseline, "model_selection").getString("verification_status"),
        )
        assertEquals(
            deviceEvidenceCurrent,
            feature(baseline, "model_selection").isNull("verification_gap"),
        )
        assertEquals("complete", feature(baseline, "disclosure_controls").getString("implementation_status"))
        assertEquals("implemented", feature(baseline, "disclosure_controls").getString("code_status"))
        assertEquals(
            if (deviceEvidenceCurrent) "device_verified" else "deferred",
            feature(baseline, "disclosure_controls").getString("verification_status"),
        )
        assertEquals(
            deviceEvidenceCurrent,
            feature(baseline, "disclosure_controls").isNull("verification_gap"),
        )
        assertEquals(
            if (deviceEvidenceCurrent) "device_verified" else "deferred",
            feature(baseline, "session_continuity_and_recovery").getString("verification_status"),
        )
        assertEquals(
            "offline_verified",
            feature(baseline, "session_long_running_stability").getString("verification_status"),
        )
        assertFalse(feature(baseline, "session_long_running_stability").isNull("verification_gap"))
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
            "official_fullscreen_fallback",
            "native_chat_composer",
            "streaming_and_stop",
            "conversation_context_paging",
            "conversation_history",
            "model_selection",
            "attachment_lifecycle",
            "composer_tools",
            "dictation",
            "realtime_voice",
            "message_actions",
            "projects",
            "tasks",
            "library",
            "gpts",
            "apps",
            "settings",
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
            "chatgpt_start_dictation",
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
            "chatgpt_open_conversation",
            "chatgpt_select_view",
        )
    }
}
