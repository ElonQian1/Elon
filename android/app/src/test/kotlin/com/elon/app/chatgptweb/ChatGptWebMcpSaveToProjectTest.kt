package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMcpSaveToProjectTest {
    @Test
    fun findsAndInvokesTheStableMessageScopedControl() {
        var invoked = ""
        val control = ChatGptWebUiControl(
            id = CONTROL_ID,
            semantic = "save_to_project",
            label = "添加至项目源",
            region = ChatGptWebUiRegion.MESSAGE,
            role = "button",
            enabled = true,
            selected = false,
            contextId = CONTEXT_ID,
        )
        val manifest = ChatGptWebUiManifest(
            version = 1,
            pageKind = "conversation",
            title = "测试会话",
            compatibility = "healthy",
            controls = listOf(control),
        )
        val actions = ChatGptWebMcpActions(
            snapshot = { null },
            uiManifest = { manifest },
            observedState = {
                ChatGptWebObservedState.Snapshot.EMPTY.copy(
                    pageGeneration = 1L,
                    adapterGeneration = 1L,
                )
            },
            beginCommand = { expectedAction ->
                ChatGptWebObservedState.CommandRequest(
                    id = "mcp_save_to_project",
                    expectedAction = expectedAction,
                    status = ChatGptWebObservedState.CommandRequest.PENDING,
                    startedAtMs = 1L,
                )
            },
            bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebModeController.Mode.NATIVE },
            inputText = { "" },
            setInputText = {},
            commands = ChatGptWebMcpTestCommandPort(onInvoke = { invoked = it }),
            refresh = {},
            selectMode = {},
            revealMessage = { _, _, _ -> false },
        )

        val found = actions.control(JSONObject()
            .put("action", "chatgpt_find_controls")
            .put("semantic", "save_to_project")
            .put("context_id", CONTEXT_ID))
        val exported = found.getJSONArray("controls").getJSONObject(0)

        assertEquals(1, found.getInt("match_count"))
        assertEquals(CONTROL_ID, exported.getString("control_id"))
        assertEquals("menu", exported.getString("native_presentation"))
        assertEquals(
            "chatgpt-message-actions:$CONTEXT_ID",
            exported.getString("native_trigger_content_description"),
        )

        val result = actions.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", CONTROL_ID))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals(CONTROL_ID, invoked)
    }

    private companion object {
        const val CONTROL_ID = "control_save_to_project_demo"
        const val CONTEXT_ID = "conversation-turn-1"
    }
}
