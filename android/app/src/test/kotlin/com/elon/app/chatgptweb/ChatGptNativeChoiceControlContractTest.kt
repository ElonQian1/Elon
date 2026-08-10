package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptNativeChoiceControlContractTest {
    @Test
    fun nativeSelectChoicesExposeStableIndexedSelectors() {
        val control = ChatGptWebUiControl(
            id = "control_model_ab12",
            semantic = "selection",
            label = "模型",
            region = ChatGptWebUiRegion.CONTENT,
            role = "combobox",
            enabled = true,
            selected = false,
            inputKind = "select",
            choiceLabels = listOf("快速", "思考"),
            selectedChoiceIndex = 0,
        )

        assertTrue(control.supportsChoiceSelection)
        assertEquals(
            "chatgpt-control-choice:control_model_ab12:1",
            ChatGptNativeChoiceControlDialog.choiceSelector(control.id, 1),
        )
    }

    @Test
    fun customComboboxWithoutPublicChoicesKeepsOfficialFallback() {
        val control = ChatGptWebUiControl(
            id = "control_model_ab12",
            semantic = "selection",
            label = "模型",
            region = ChatGptWebUiRegion.CONTENT,
            role = "combobox",
            enabled = true,
            selected = false,
            inputKind = "select",
        )

        assertFalse(control.supportsChoiceSelection)
    }
}
