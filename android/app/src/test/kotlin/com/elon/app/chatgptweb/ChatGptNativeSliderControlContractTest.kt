package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptNativeSliderControlContractTest {
    @Test
    fun nativeRangeExposesStableSliderValueAndCommitSelectors() {
        val control = ChatGptWebUiControl(
            id = "control_effort_ab12",
            semantic = "slider",
            label = "思考强度",
            region = ChatGptWebUiRegion.CONTENT,
            role = "slider",
            enabled = true,
            selected = false,
            inputKind = "range",
            slider = ChatGptWebSlider(0.0, 2.0, 0.5, 1.0),
        )

        assertTrue(control.supportsSliderValue)
        assertEquals(4, control.slider?.stepCount)
        assertEquals(
            "chatgpt-control-slider:control_effort_ab12",
            ChatGptNativeSliderControlDialog.sliderSelector(control.id),
        )
        assertEquals(
            "chatgpt-control-slider-value:control_effort_ab12",
            ChatGptNativeSliderControlDialog.valueSelector(control.id),
        )
        assertEquals(
            "chatgpt-control-slider-commit:control_effort_ab12",
            ChatGptNativeSliderControlDialog.commitSelector(control.id),
        )
    }

    @Test
    fun ariaSliderWithoutNativeRangeMetadataKeepsOfficialFallback() {
        val control = ChatGptWebUiControl(
            id = "control_effort_ab12",
            semantic = "slider",
            label = "思考强度",
            region = ChatGptWebUiRegion.CONTENT,
            role = "slider",
            enabled = true,
            selected = false,
            inputKind = "range",
        )

        assertFalse(control.supportsSliderValue)
    }
}
