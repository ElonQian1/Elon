package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionAdaptiveControlsTest {
    @Test
    fun decodesWritableTextAndStableInputMetadata() {
        val control = WebChatProductionControlJson.parse(base("name", "text_input", "textbox")
            .put("input_kind", "text")
            .put("writable", true))

        assertNotNull(control)
        assertTrue(requireNotNull(control).supportsTextEntry)
        assertEquals("text", control.inputKind)
    }

    @Test
    fun decodesChoicesSelectedStateExpansionAndSlider() {
        val choice = requireNotNull(WebChatProductionControlJson.parse(
            base("choice", "selection", "combobox")
                .put("input_kind", "select")
                .put("choice_labels", JSONArray(listOf("快速", "深入")))
                .put("selected_choice_index", 1),
        ))
        val toggle = requireNotNull(WebChatProductionControlJson.parse(
            base("toggle", "toggle", "switch")
                .put("state_settable", true)
                .put("selected", true),
        ))
        val expanded = requireNotNull(WebChatProductionControlJson.parse(
            base("expanded", "reasoning_details", "button")
                .put("expandable", true)
                .put("expanded", false),
        ))
        val slider = requireNotNull(WebChatProductionControlJson.parse(
            base("slider", "slider", "slider")
                .put("input_kind", "range")
                .put("slider", JSONObject()
                    .put("min", 0.0)
                    .put("max", 10.0)
                    .put("step", 0.5)
                    .put("value", 4.5)),
        ))

        assertTrue(choice.supportsChoiceSelection)
        assertEquals(1, choice.selectedChoiceIndex)
        assertTrue(toggle.supportsSelectedState)
        assertTrue(expanded.supportsExpandedState)
        assertTrue(slider.supportsSliderValue)
        assertEquals(4.5, requireNotNull(slider.slider).value, 0.0)
    }

    @Test
    fun rejectsMalformedControlsAndUnsafeSliders() {
        assertNull(WebChatProductionControlJson.parse(JSONObject().put("control_id", "missing")))
        val slider = WebChatProductionControlJson.parse(
            base("bad_slider", "slider", "slider")
                .put("input_kind", "range")
                .put("slider", JSONObject()
                    .put("min", 0.0)
                    .put("max", 1.0)
                    .put("step", 0.0)
                    .put("value", 0.5)),
        )
        assertNotNull(slider)
        assertFalse(requireNotNull(slider).supportsSliderValue)

        val excessive = WebChatProductionControlJson.parse(
            base("excessive_slider", "slider", "slider")
                .put("input_kind", "range")
                .put("slider", JSONObject()
                    .put("min", 0.0)
                    .put("max", 1_000_000.0)
                    .put("step", 0.001)
                    .put("value", 1.0)),
        )
        assertNotNull(excessive)
        assertFalse(requireNotNull(excessive).supportsSliderValue)
    }

    @Test
    fun treatsExplicitJsonNullAsAbsentMetadata() {
        val control = requireNotNull(WebChatProductionControlJson.parse(
            base("null_metadata", "action", "button")
                .put("input_kind", JSONObject.NULL)
                .put("context_id", JSONObject.NULL),
        ))

        assertNull(control.inputKind)
        assertNull(control.contextId)
    }

    @Test
    fun exposesStableSelectorsForBooleanAndDisclosureControls() {
        assertEquals(
            "web-chat-control-state:control_toggle",
            WebChatProductionAdaptiveControlSelectors.stateList("control_toggle"),
        )
        assertEquals(
            "web-chat-control-state:control_toggle:true",
            WebChatProductionAdaptiveControlSelectors.stateValue("control_toggle", true),
        )
        assertEquals(
            "web-chat-control-expanded:control_more:false",
            WebChatProductionAdaptiveControlSelectors.expansionValue("control_more", false),
        )
    }

    private fun base(id: String, semantic: String, role: String): JSONObject = JSONObject()
        .put("control_id", "control_$id")
        .put("semantic", semantic)
        .put("label", id)
        .put("region", "overlay")
        .put("role", role)
        .put("enabled", true)
}
