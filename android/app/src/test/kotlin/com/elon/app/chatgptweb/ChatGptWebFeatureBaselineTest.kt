package com.elon.app.chatgptweb

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

        assertEquals("elon.chatgpt_web.feature_baseline.v1", baseline.getString("schema"))
        assertEquals(ChatGptWebFeatureBaseline.VERSION, baseline.getInt("version"))
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
        }
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
