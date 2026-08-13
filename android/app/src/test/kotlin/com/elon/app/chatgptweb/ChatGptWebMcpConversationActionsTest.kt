package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebMcpConversationActionsTest {
    @Test
    fun projectConversationPathsAreExposedAndCanBeOpened() {
        var openedPath = ""
        val conversation = ChatGptWebConversation(
            id = "project-chat",
            title = "项目会话",
            path = "/g/g-p-demo/c/project-chat",
            active = true,
            groupLabel = "今天",
            projectId = "g-p-demo",
            projectTitle = "安卓项目",
            projectPath = "/g/g-p-demo/project",
            activityDates = setOf("2026-08-14"),
        )
        var nextCommandId = 0
        val actions = ChatGptWebMcpActions(
            snapshot = { null },
            uiManifest = { null },
            observedState = {
                ChatGptWebObservedState.Snapshot(
                    conversations = listOf(conversation),
                    features = emptyList(),
                    composerSections = emptyMap(),
                    lastCommand = null,
                    commandRequests = emptyList(),
                    updatedAtMs = 123L,
                    pageGeneration = 1L,
                    adapterGeneration = 1L,
                    projects = listOf(
                        ChatGptWebProject("g-p-demo", "安卓项目", "/g/g-p-demo/project"),
                    ),
                )
            },
            beginCommand = { expectedAction ->
                ChatGptWebObservedState.CommandRequest(
                    id = "mcp_${++nextCommandId}",
                    expectedAction = expectedAction,
                    status = ChatGptWebObservedState.CommandRequest.PENDING,
                    startedAtMs = 123L,
                )
            },
            bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebModeController.Mode.NATIVE },
            inputText = { "" },
            setInputText = {},
            commands = ChatGptWebMcpTestCommandPort(onOpenConversation = { openedPath = it }),
            refresh = {},
            selectMode = {},
            revealMessage = { _, _, _ -> false },
        )

        val listed = actions.control(JSONObject().put("action", "chatgpt_get_conversations"))
        val item = listed.getJSONArray("conversations").getJSONObject(0)
        assertEquals(1, listed.getInt("project_count"))
        assertEquals("g-p-demo", item.getString("project_id"))
        assertEquals("今天", item.getString("group_label"))
        assertEquals("2026-08-14", item.getJSONArray("activity_dates").getString(0))

        actions.control(JSONObject()
            .put("action", "chatgpt_open_conversation")
            .put("conversation_path", conversation.path))
        assertEquals(conversation.path, openedPath)
    }
}
