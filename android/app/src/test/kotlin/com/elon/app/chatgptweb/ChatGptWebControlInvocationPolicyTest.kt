package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebControlInvocationPolicyTest {
    @Test
    fun highImpactAndUnknownActionsRequireExplicitUserConfirmation() {
        listOf(
            "action",
            "archive",
            "branch",
            "confirm",
            "delete",
            "dictation",
            "download_app",
            "edit",
            "feedback",
            "logout",
            "personalization",
            "pin",
            "plan",
            "read_aloud",
            "rename",
            "save_to_project",
            "share",
            "voice_mode",
        ).forEach { semantic ->
            assertEquals(
                ChatGptWebControlInvocationPolicy.Risk.USER_CONFIRMATION,
                ChatGptWebControlInvocationPolicy.risk(semantic),
            )
        }
    }

    @Test
    fun readOnlyAndOrdinaryConversationControlsRemainDirectlyInvokable() {
        listOf("copy", "model", "navigation", "search", "suggestion").forEach { semantic ->
            assertEquals(
                ChatGptWebControlInvocationPolicy.Risk.STANDARD,
                ChatGptWebControlInvocationPolicy.risk(semantic),
            )
        }
        assertNull(
            ChatGptWebControlInvocationPolicy.rejection(
                control("suggestion"),
                userConfirmed = false,
            ),
        )
    }

    @Test
    fun explicitConfirmationUnlocksOnlyTheRequestedCurrentControl() {
        val control = control("delete")

        assertEquals(
            "user_confirmation_required",
            ChatGptWebControlInvocationPolicy.rejection(control, userConfirmed = false),
        )
        assertNull(ChatGptWebControlInvocationPolicy.rejection(control, userConfirmed = true))
    }

    private fun control(semantic: String) = ChatGptWebUiControl(
        id = "control_demo",
        semantic = semantic,
        label = "Demo",
        region = ChatGptWebUiRegion.OVERLAY,
        role = "button",
        enabled = true,
        selected = false,
    )
}
