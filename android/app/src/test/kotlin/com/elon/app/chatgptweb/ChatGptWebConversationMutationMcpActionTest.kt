package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebConversationMutationMcpActionTest {
    @Test
    fun pinRequiresConfirmationAndDispatchesOneNormalizedMutation() {
        var target: Pair<String, Boolean>? = null
        val actions = ChatGptWebMcpActionsTest().actions(
            onSetConversationPinned = { path, pinned -> target = path to pinned },
        )

        val missingConfirmation = actions.control(JSONObject()
            .put("action", "chatgpt_set_conversation_pinned")
            .put("conversation_path", "/c/demo")
            .put("pinned", true))
        val invalidPath = actions.control(JSONObject()
            .put("action", "chatgpt_set_conversation_pinned")
            .put("conversation_path", "https://example.com/c/demo")
            .put("pinned", true)
            .put("user_confirmed", true))
        val accepted = actions.control(JSONObject()
            .put("action", "chatgpt_set_conversation_pinned")
            .put("conversation_path", "/g/g-p-demo/c/demo")
            .put("pinned", false)
            .put("user_confirmed", true))

        assertFalse(missingConfirmation.getBoolean("control_ok"))
        assertEquals("user_confirmation_required", missingConfirmation.getString("error"))
        assertFalse(invalidPath.getBoolean("control_ok"))
        assertEquals("invalid_conversation_path", invalidPath.getString("error"))
        assertTrue(accepted.getBoolean("control_ok"))
        assertEquals("/g/g-p-demo/c/demo" to false, target)
        assertEquals(
            "set_conversation_pinned",
            accepted.getJSONObject("command_receipt").getString("expected_web_action"),
        )
    }
}
