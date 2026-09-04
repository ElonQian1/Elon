package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebConversationMutationMcpActionTest {
    @Test
    fun mutationsRequireConfirmationAndDispatchTypedNormalizedCommands() {
        var pinnedTarget: Pair<String, Boolean>? = null
        var archivedTarget: Pair<String, Boolean>? = null
        var renamedTarget: Pair<String, String>? = null
        val actions = ChatGptWebMcpActionsTest().actions(
            onSetConversationPinned = { path, pinned -> pinnedTarget = path to pinned },
            onSetConversationArchived = { path, archived -> archivedTarget = path to archived },
            onRenameConversation = { path, title -> renamedTarget = path to title },
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
        val acceptedPin = actions.control(JSONObject()
            .put("action", "chatgpt_set_conversation_pinned")
            .put("conversation_path", "/g/g-p-demo/c/demo")
            .put("pinned", false)
            .put("user_confirmed", true))
        val acceptedArchive = actions.control(JSONObject()
            .put("action", "chatgpt_set_conversation_archived")
            .put("conversation_path", "/c/demo")
            .put("archived", true)
            .put("user_confirmed", true))
        val acceptedRename = actions.control(JSONObject()
            .put("action", "chatgpt_rename_conversation")
            .put("conversation_path", "/c/demo")
            .put("title", "新的会话标题")
            .put("user_confirmed", true))

        assertFalse(missingConfirmation.getBoolean("control_ok"))
        assertEquals("user_confirmation_required", missingConfirmation.getString("error"))
        assertFalse(invalidPath.getBoolean("control_ok"))
        assertEquals("invalid_conversation_path", invalidPath.getString("error"))
        assertTrue(acceptedPin.getBoolean("control_ok"))
        assertEquals("/g/g-p-demo/c/demo" to false, pinnedTarget)
        assertEquals(
            "set_conversation_pinned",
            acceptedPin.getJSONObject("command_receipt").getString("expected_web_action"),
        )
        assertTrue(acceptedArchive.getBoolean("control_ok"))
        assertEquals("/c/demo" to true, archivedTarget)
        assertEquals(
            "set_conversation_archived",
            acceptedArchive.getJSONObject("command_receipt").getString("expected_web_action"),
        )
        assertTrue(acceptedRename.getBoolean("control_ok"))
        assertEquals("/c/demo" to "新的会话标题", renamedTarget)
        assertEquals(
            "rename_conversation",
            acceptedRename.getJSONObject("command_receipt").getString("expected_web_action"),
        )
    }

    @Test
    fun renameRejectsBlankOrOversizedTitlesBeforeDispatch() {
        var dispatchCount = 0
        val actions = ChatGptWebMcpActionsTest().actions(
            onRenameConversation = { _, _ -> dispatchCount += 1 },
        )

        listOf("   ", "x".repeat(161)).forEach { title ->
            val result = actions.control(JSONObject()
                .put("action", "chatgpt_rename_conversation")
                .put("conversation_path", "/c/demo")
                .put("title", title)
                .put("user_confirmed", true))
            assertFalse(result.getBoolean("control_ok"))
            assertEquals("invalid_title", result.getString("error"))
        }
        assertEquals(0, dispatchCount)
    }
}
