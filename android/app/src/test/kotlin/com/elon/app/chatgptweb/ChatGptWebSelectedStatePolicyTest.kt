package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebSelectedStatePolicyTest {
    @Test
    fun permitsDesiredStateForSwitchesAndRejectsClearingExclusiveControls() {
        assertNull(ChatGptWebSelectedStatePolicy.rejection(control("switch", false), true))
        assertNull(ChatGptWebSelectedStatePolicy.rejection(control("switch", true), false))
        assertNull(ChatGptWebSelectedStatePolicy.rejection(control("tab", true), true))
        assertEquals(
            "tab_cannot_be_cleared",
            ChatGptWebSelectedStatePolicy.rejection(control("tab", true), false),
        )
        assertEquals(
            "radio_cannot_be_cleared",
            ChatGptWebSelectedStatePolicy.rejection(control("menuitemradio", true), false),
        )
    }

    @Test
    fun rejectsControlsWithoutSettableState() {
        val control = control("button", false, stateSettable = false)

        assertEquals(
            "control_state_not_settable",
            ChatGptWebSelectedStatePolicy.rejection(control, true),
        )
    }

    private fun control(
        role: String,
        selected: Boolean,
        stateSettable: Boolean = true,
    ) = ChatGptWebUiControl(
        id = "control_${role}_demo",
        semantic = if (role == "tab") "selection" else "toggle",
        label = "测试控件",
        region = ChatGptWebUiRegion.OVERLAY,
        role = role,
        enabled = true,
        selected = selected,
        inputKind = role,
        stateSettable = stateSettable,
    )
}
