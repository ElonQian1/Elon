package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMcpActionsTest {
    @Test
    fun stateExportsConversationContextAndStableControlMetadata() {
        val actions = actions()

        val state = actions.uiState()
        val conversation = state.getJSONObject("conversation")
        val control = state.getJSONObject("ui_manifest").getJSONArray("controls").getJSONObject(0)

        assertEquals("chatgpt_web", state.getString("surface"))
        assertEquals("完整回答内容", conversation.getJSONArray("messages").getJSONObject(0).getString("content"))
        assertEquals("control_suggestion_demo", control.getString("control_id"))
        assertEquals(0.25, control.getDouble("web_x_ratio"), 0.0)
        assertEquals(
            "chatgpt-control:control_suggestion_demo:整理待办",
            control.getString("adb_content_description"),
        )
    }

    @Test
    fun controlInvokesOnlyIdsPresentInTheCurrentManifest() {
        var invoked = ""
        val actions = actions(onInvoke = { invoked = it })

        val ok = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_suggestion_demo"))
        val stale = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", "control_suggestion_stale"))

        assertTrue(ok.getBoolean("control_ok"))
        assertEquals("control_suggestion_demo", invoked)
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("stale_control_id", stale.getString("error"))
    }

    @Test
    fun contextCanBeReadInStablePages() {
        val result = actions().control(JSONObject()
            .put("action", "chatgpt_get_context")
            .put("message_offset", 0)
            .put("message_limit", 1))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals(1, result.getInt("message_count"))
        assertEquals(0, result.getJSONArray("messages").getJSONObject(0).getInt("index"))
        assertEquals("完整回答内容", result.getJSONArray("messages").getJSONObject(0).getString("content"))
        assertFalse(result.getBoolean("has_more"))
    }

    private fun actions(onInvoke: (String) -> Unit = {}): ChatGptWebMcpActions {
        val snapshot = ChatGptWebSnapshot(
            title = "工作",
            url = "https://chatgpt.com/c/demo",
            draft = "",
            messages = listOf(ChatGptWebMessage("a1", "assistant", "完整回答内容", "completed", emptyList())),
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "5.6 Sol 轻度",
            attachments = emptyList(),
            dictationActive = false,
            capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.DRAFT_SYNC)),
        )
        val manifest = ChatGptWebUiManifest(
            version = 1,
            pageKind = "conversation",
            title = "工作",
            compatibility = "healthy",
            controls = listOf(
                ChatGptWebUiControl(
                    id = "control_suggestion_demo",
                    semantic = "suggestion",
                    label = "整理待办",
                    region = ChatGptWebUiRegion.SUGGESTIONS,
                    role = "button",
                    enabled = true,
                    selected = false,
                    webXRatio = 0.25,
                    webYRatio = 0.75,
                ),
            ),
        )
        return ChatGptWebMcpActions(
            snapshot = { snapshot },
            uiManifest = { manifest },
            bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebModeController.Mode.NATIVE },
            inputText = { "" },
            setInputText = {},
            sendInput = {},
            invokeControl = onInvoke,
            newConversation = {},
            stopGeneration = {},
            refresh = {},
            refreshControls = {},
            selectMode = {},
            openConversation = {},
            listConversations = {},
        )
    }
}
