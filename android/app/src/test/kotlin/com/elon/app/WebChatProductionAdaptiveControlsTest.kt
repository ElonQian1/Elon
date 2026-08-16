package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebSlider
import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionAdaptiveControlsTest {
    @Test
    fun usesTypedWritableTextAndStableInputMetadata() {
        val control = control(
            id = "name",
            semantic = "text_input",
            role = "textbox",
            inputKind = "text",
            writable = true,
        )

        assertTrue(control.supportsTextEntry)
        assertEquals("text", control.inputKind)
    }

    @Test
    fun supportsTypedChoicesSelectedStateExpansionAndSlider() {
        val choice = control(
            id = "choice",
            semantic = "selection",
            role = "combobox",
            inputKind = "select",
            choiceLabels = listOf("快速", "深入"),
            selectedChoiceIndex = 1,
        )
        val toggle = control("toggle", "toggle", "switch", stateSettable = true, selected = true)
        val expanded = control(
            "expanded",
            "reasoning_details",
            "button",
            expandable = true,
            expanded = false,
        )
        val slider = control(
            "slider",
            "slider",
            "slider",
            inputKind = "range",
            slider = ChatGptWebSlider(0.0, 10.0, 0.5, 4.5),
        )

        assertTrue(choice.supportsChoiceSelection)
        assertEquals(1, choice.selectedChoiceIndex)
        assertTrue(toggle.supportsSelectedState)
        assertTrue(expanded.supportsExpandedState)
        assertTrue(slider.supportsSliderValue)
        assertEquals(4.5, requireNotNull(slider.slider).value, 0.0)
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

    private fun control(
        id: String,
        semantic: String,
        role: String,
        inputKind: String? = null,
        writable: Boolean = false,
        stateSettable: Boolean = false,
        selected: Boolean = false,
        choiceLabels: List<String> = emptyList(),
        selectedChoiceIndex: Int? = null,
        slider: ChatGptWebSlider? = null,
        expandable: Boolean = false,
        expanded: Boolean? = null,
    ) = ChatGptWebUiControl(
        id = "control_$id",
        semantic = semantic,
        label = id,
        region = "overlay",
        role = role,
        enabled = true,
        selected = selected,
        inputKind = inputKind,
        writable = writable,
        stateSettable = stateSettable,
        choiceLabels = choiceLabels,
        selectedChoiceIndex = selectedChoiceIndex,
        slider = slider,
        expandable = expandable,
        expanded = expanded,
    )
}
