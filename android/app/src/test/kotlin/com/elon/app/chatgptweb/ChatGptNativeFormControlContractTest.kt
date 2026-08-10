package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptNativeFormControlContractTest {
    @Test
    fun writableTextControlsExposeStableInputAndCommitSelectors() {
        val control = ChatGptWebUiControl(
            id = "control_search_ab12",
            semantic = "search",
            label = "搜索聊天",
            region = ChatGptWebUiRegion.CONTENT,
            role = "textbox",
            enabled = true,
            selected = false,
            inputKind = "search",
            writable = true,
        )

        assertTrue(control.supportsTextEntry)
        assertEquals(
            "chatgpt-control-input:control_search_ab12",
            ChatGptNativeFormControlDialog.inputSelector(control.id),
        )
        assertEquals(
            "chatgpt-control-input-commit:control_search_ab12",
            ChatGptNativeFormControlDialog.commitSelector(control.id),
        )
    }

    @Test
    fun passwordControlsNeverBecomeNativeTextEntryControls() {
        val control = ChatGptWebUiControl(
            id = "control_password_ab12",
            semantic = "text_input",
            label = "密码",
            region = ChatGptWebUiRegion.CONTENT,
            role = "textbox",
            enabled = true,
            selected = false,
            inputKind = "password",
            writable = true,
        )

        assertFalse(control.supportsTextEntry)
    }
}
