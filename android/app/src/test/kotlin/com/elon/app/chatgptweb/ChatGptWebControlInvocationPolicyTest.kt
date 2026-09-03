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

    @Test
    fun touchMissFallbackIsLimitedToTheCurrentHeaderConversationMenu() {
        val current = control(
            semantic = "conversation_options",
            region = ChatGptWebUiRegion.HEADER,
            contextId = "conversation-id",
        )
        val projectUrl =
            "https://chatgpt.com/g/g-p-1234567890abcdef1234567890abcdef/c/conversation-id"

        assertNull(ChatGptWebControlInvocationPolicy.afterTouchMissRejection(
            current,
            projectUrl,
            listOf(current),
        ))
        assertEquals(
            "touch_miss_fallback_context_changed",
            ChatGptWebControlInvocationPolicy.afterTouchMissRejection(
                current.copy(contextId = "other-conversation"),
                projectUrl,
                listOf(current),
            ),
        )
        assertEquals(
            "touch_miss_fallback_overlay_present",
            ChatGptWebControlInvocationPolicy.afterTouchMissRejection(
                current,
                projectUrl,
                listOf(current, control("project")),
            ),
        )
        assertEquals(
            "touch_miss_fallback_not_supported",
            ChatGptWebControlInvocationPolicy.afterTouchMissRejection(
                current.copy(region = ChatGptWebUiRegion.CONTENT),
                projectUrl,
                listOf(current),
            ),
        )
    }

    private fun control(
        semantic: String,
        region: String = ChatGptWebUiRegion.OVERLAY,
        contextId: String? = null,
    ) = ChatGptWebUiControl(
        id = "control_demo",
        semantic = semantic,
        label = "Demo",
        region = region,
        role = "button",
        enabled = true,
        selected = false,
        contextId = contextId,
    )
}
