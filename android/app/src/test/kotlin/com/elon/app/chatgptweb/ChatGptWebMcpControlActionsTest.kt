package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMcpControlActionsTest {
    private val fixture = ChatGptWebMcpActionsTest()

    @Test
    fun touchMissFallbackOnlyTargetsTheCurrentProjectConversationMenu() {
        var invoked = ""
        var dispatched = ""
        val projectUrl =
            "https://chatgpt.com/g/g-p-1234567890abcdef1234567890abcdef/c/conversation-demo"
        val actions = fixture.actions(
            snapshotUrl = projectUrl,
            includeProjectConversationControl = true,
            onInvoke = { invoked = it },
            onDispatch = { action, _ -> dispatched = action },
        )

        val accepted = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_project_conversation_options")
            .put("after_touch_miss", true))

        assertTrue(accepted.getBoolean("control_ok"))
        assertEquals("control_project_conversation_options", invoked)
        assertEquals("invoke_ui_control_after_touch_miss", dispatched)
        assertEquals(
            "invoke_ui_control_after_touch_miss",
            accepted.getJSONObject("command_receipt").getString("expected_web_action"),
        )

        val stale = fixture.actions(
            snapshotUrl = projectUrl.replace("conversation-demo", "other-conversation"),
            includeProjectConversationControl = true,
        ).control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_project_conversation_options")
            .put("after_touch_miss", true))
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("touch_miss_fallback_context_changed", stale.getString("error"))

        val overlay = fixture.actions(
            snapshotUrl = projectUrl,
            includeProjectConversationControl = true,
            includeOverlayControl = true,
        ).control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_project_conversation_options")
            .put("after_touch_miss", true))
        assertFalse(overlay.getBoolean("control_ok"))
        assertEquals("touch_miss_fallback_overlay_present", overlay.getString("error"))
    }

    @Test
    fun stateAndChoiceControlsDispatchIdempotentCommandsWithoutPrivateValues() {
        var selectedTarget: Pair<String, Boolean>? = null
        var choiceTarget: Pair<String, Int>? = null
        val actions = fixture.actions(
            includeFormControls = true,
            onSetControlSelected = { id, selected -> selectedTarget = id to selected },
            onSelectControlChoice = { id, index -> choiceTarget = id to index },
        )

        val selected = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_selected")
            .put("control_id", "control_toggle_demo")
            .put("selected", true))
        val choice = actions.control(JSONObject()
            .put("action", "chatgpt_select_control_choice")
            .put("control_id", "control_model_demo")
            .put("choice_index", 1))
        val controls = actions.uiState().getJSONObject("ui_manifest").getJSONArray("controls")
        val toggle = controls.getJSONObject(2)
        val model = controls.getJSONObject(3)

        assertTrue(selected.getBoolean("control_ok"))
        assertEquals("control_toggle_demo" to true, selectedTarget)
        assertTrue(choice.getBoolean("control_ok"))
        assertEquals("control_model_demo" to 1, choiceTarget)
        assertTrue(toggle.getBoolean("state_settable"))
        assertEquals(2, model.getJSONArray("choice_labels").length())
        assertEquals(0, model.getInt("selected_choice_index"))
        assertFalse(model.has("value"))
        assertEquals(
            "chatgpt-control-choice:control_model_demo:1",
            model.getJSONArray("native_choice_content_descriptions").getString(1),
        )

        val missingState = actions.control(JSONObject()
            .put("action", "chatgpt_set_control_selected")
            .put("control_id", "control_toggle_demo"))
        val fractionalChoice = actions.control(JSONObject()
            .put("action", "chatgpt_select_control_choice")
            .put("control_id", "control_model_demo")
            .put("choice_index", 1.5))
        assertFalse(missingState.getBoolean("control_ok"))
        assertEquals("missing_selected", missingState.getString("error"))
        assertFalse(fractionalChoice.getBoolean("control_ok"))
        assertEquals("invalid_choice_index", fractionalChoice.getString("error"))
    }
}
