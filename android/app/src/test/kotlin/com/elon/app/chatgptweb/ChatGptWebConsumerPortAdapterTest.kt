package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerCommandStatus
import com.elon.app.WebChatConsumerControlMutation
import com.elon.app.WebChatConsumerControlPresentation
import com.elon.app.WebChatConsumerPageActionPlacement
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebConsumerPortAdapterTest {
    @Test
    fun exposesTypedComposerStateWithoutReadingMcpJson() {
        val observed = ChatGptWebObservedState.Snapshot.EMPTY.copy(
            pageGeneration = 3,
            adapterGeneration = 3,
            features = listOf(ChatGptWebFeature(
                id = "health",
                label = "Health",
                kind = "health",
                selected = false,
            )),
            composerSections = mapOf(
                "tools" to listOf(ChatGptWebComposerOption(
                    id = "search",
                    label = "Search",
                    selected = true,
                    kind = "tool",
                    semantic = "web_search",
                    opensSubmenu = false,
                    parentId = "tools_parent",
                    parentLabel = "Tools",
                )),
            ),
            commandRequests = listOf(ChatGptWebObservedState.CommandRequest(
                id = "mcp_0",
                expectedAction = "select_navigation",
                status = ChatGptWebObservedState.CommandRequest.SUCCEEDED,
                startedAtMs = 1L,
                completedAtMs = 2L,
            )),
        )
        val port = ChatGptWebConsumerPortAdapter(
            snapshot = {
                snapshot(
                    streaming = true,
                    dictationActive = false,
                    dictationCaptureActive = true,
                    dictationCapturePending = false,
                    pageKind = "health",
                    url = "https://chatgpt.com/health",
                    draft = "pending",
                    privateReadAloudReady = true,
                    privateReadAloudState = "playing",
                    privateReadAloudContextId = "assistant-1",
                )
            },
            uiManifest = { manifest() },
            observedState = { observed },
            executeControl = { error("state reads must not use the MCP JSON port") },
        )

        val state = port.state()

        assertTrue(state.streaming)
        assertTrue(state.adapterCurrent)
        assertFalse(state.dictationActive)
        assertTrue(state.dictationCaptureActive)
        assertFalse(state.dictationCapturePending)
        assertTrue(state.draftPresent)
        assertTrue(state.privateReadAloudReady)
        assertEquals("playing", state.privateReadAloudState)
        assertEquals("assistant-1", state.privateReadAloudContextId)
        assertEquals("health", state.pageKind)
        assertEquals("https://chatgpt.com/health", state.pageUrl)
        assertEquals("search", state.composerSections.getValue("tools").single().id)
        assertEquals("tools_parent", state.composerSections.getValue("tools").single().parentId)
        assertEquals("Tools", state.composerSections.getValue("tools").single().parentLabel)
        assertTrue(state.composerSections.getValue("tools").single().nativeSelector.contains("search"))
        assertTrue(state.features.single().requiresUserConfirmation)
        assertTrue(state.features.single().nativeSelector.contains("health"))
        assertEquals("temporary", state.controls.single().control.id)
        assertEquals(WebChatConsumerControlPresentation.DIRECT, state.controls.single().presentation)
        assertEquals(
            WebChatConsumerPageActionPlacement.NONE,
            state.controls.single().pageActionPlacement,
        )
        assertEquals(WebChatConsumerCommandStatus.SUCCEEDED, state.commandRequests.single().status)
    }

    @Test
    fun hidesStaleTypedStateWhenTheAdapterGenerationChanged() {
        val observed = ChatGptWebObservedState.Snapshot.EMPTY.copy(
            pageGeneration = 4,
            adapterGeneration = 3,
            composerSections = mapOf("tools" to emptyList()),
        )
        val port = ChatGptWebConsumerPortAdapter(
            snapshot = { snapshot(streaming = true, dictationActive = true) },
            uiManifest = { manifest() },
            observedState = { observed },
            executeControl = { JSONObject() },
        )

        val state = port.state()

        assertFalse(state.streaming)
        assertFalse(state.adapterCurrent)
        assertFalse(state.dictationActive)
        assertFalse(state.draftPresent)
        assertTrue(state.composerSections.isEmpty())
        assertTrue(state.features.isEmpty())
        assertTrue(state.controls.isEmpty())
        assertEquals("unknown", state.pageKind)
        assertEquals("", state.pageUrl)
    }

    @Test
    fun mapsConsumerCommandsToTheExistingControlExecutor() {
        val requests = mutableListOf<JSONObject>()
        val port = ChatGptWebConsumerPortAdapter(
            snapshot = { null },
            uiManifest = { null },
            observedState = { ChatGptWebObservedState.Snapshot.EMPTY },
            executeControl = { args ->
                requests += JSONObject(args.toString())
                JSONObject()
                    .put("control_ok", true)
                    .put("command_receipt", JSONObject().put("request_id", "mcp_1"))
            },
        )

        val requested = port.requestComposerOptions("tools")
        val selected = port.selectComposerOption("tools", "search")
        val features = port.requestFeatures()
        val feature = port.selectFeature("health", userConfirmed = true)
        val controls = port.requestControls()
        val revealed = port.revealProjectChoice("Project Alpha")
        val invoked = port.invokeControl("temporary", userConfirmed = false)
        val retried = port.invokeControlAfterTouchMiss("temporary", userConfirmed = false)
        val readAloud = port.toggleOfficialReadAloud("assistant-1")
        val updated = port.updateControl(
            "temporary",
            WebChatConsumerControlMutation.Selected(true),
        )
        val dictation = port.executeSessionCommand("chatgpt_start_dictation")
        val cancelDictation = port.executeSessionCommand("chatgpt_cancel_dictation")
        val submitDictation = port.executeSessionCommand("chatgpt_submit_dictation")
        val prepareRealtimeVoice = port.executeSessionCommand("chatgpt_prepare_realtime_voice")
        val realtimeVoice = port.executeSessionCommand("chatgpt_start_realtime_voice")
        val dismissed = port.dismissComposerOptions()
        val moved = port.moveConversationToProject(
            conversationPath = "/c/demo",
            conversationTitle = "项目会话",
            projectId = "g-p-demo",
            userConfirmed = true,
        )
        val unsupported = port.executeSessionCommand("chatgpt_delete_account")

        assertEquals("chatgpt_list_composer_options", requests[0].getString("action"))
        assertEquals("tools", requests[0].getString("section"))
        assertEquals("search", requests[1].getString("option_id"))
        assertEquals("chatgpt_list_features", requests[2].getString("action"))
        assertEquals("health", requests[3].getString("feature_id"))
        assertTrue(requests[3].getBoolean("user_confirmed"))
        assertEquals("chatgpt_refresh_controls", requests[4].getString("action"))
        assertEquals("chatgpt_reveal_project_choice", requests[5].getString("action"))
        assertEquals("Project Alpha", requests[5].getString("project_title"))
        assertEquals("temporary", requests[6].getString("control_id"))
        assertEquals("chatgpt_invoke_control", requests[7].getString("action"))
        assertTrue(requests[7].getBoolean("after_touch_miss"))
        assertEquals("chatgpt_toggle_private_read_aloud", requests[8].getString("action"))
        assertEquals("assistant-1", requests[8].getString("context_id"))
        assertEquals("chatgpt_set_control_selected", requests[9].getString("action"))
        assertTrue(requests[9].getBoolean("selected"))
        assertEquals("chatgpt_start_dictation", requests[10].getString("action"))
        assertEquals("chatgpt_cancel_dictation", requests[11].getString("action"))
        assertEquals("chatgpt_submit_dictation", requests[12].getString("action"))
        assertEquals("chatgpt_prepare_realtime_voice", requests[13].getString("action"))
        assertEquals("chatgpt_start_realtime_voice", requests[14].getString("action"))
        assertEquals("chatgpt_dismiss_composer_options", requests[15].getString("action"))
        assertEquals("chatgpt_move_conversation_to_project", requests[16].getString("action"))
        assertEquals("/c/demo", requests[16].getString("conversation_path"))
        assertEquals("项目会话", requests[16].getString("conversation_title"))
        assertEquals("g-p-demo", requests[16].getString("project_id"))
        assertTrue(requests[16].getBoolean("user_confirmed"))
        assertTrue(requested.accepted)
        assertTrue(selected.accepted)
        assertTrue(features.accepted)
        assertTrue(feature.accepted)
        assertTrue(controls.accepted)
        assertTrue(revealed.accepted)
        assertTrue(invoked.accepted)
        assertTrue(retried.accepted)
        assertTrue(readAloud.accepted)
        assertTrue(updated.accepted)
        assertEquals("mcp_1", dictation.requestId)
        assertEquals("mcp_1", cancelDictation.requestId)
        assertEquals("mcp_1", submitDictation.requestId)
        assertEquals("mcp_1", prepareRealtimeVoice.requestId)
        assertEquals("mcp_1", realtimeVoice.requestId)
        assertTrue(dismissed.accepted)
        assertTrue(moved.accepted)
        assertFalse(unsupported.accepted)
        assertEquals("unsupported_consumer_command", unsupported.error)
        assertNull(unsupported.requestId)
    }

    private fun snapshot(
        streaming: Boolean,
        dictationActive: Boolean,
        dictationCaptureActive: Boolean = false,
        dictationCapturePending: Boolean = false,
        pageKind: String = "unknown",
        url: String = "https://chatgpt.com/",
        draft: String = "",
        privateReadAloudReady: Boolean = false,
        privateReadAloudState: String = "idle",
        privateReadAloudContextId: String = "",
    ) = ChatGptWebSnapshot(
        title = "",
        url = url,
        draft = draft,
        messages = emptyList(),
        authenticated = false,
        composerReady = true,
        streaming = streaming,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = dictationActive,
        dictationCaptureActive = dictationCaptureActive,
        dictationCapturePending = dictationCapturePending,
        privateReadAloudReady = privateReadAloudReady,
        privateReadAloudState = privateReadAloudState,
        privateReadAloudContextId = privateReadAloudContextId,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = pageKind,
    )

    private fun manifest() = ChatGptWebUiManifest(
        version = 1,
        pageKind = "conversation",
        title = "",
        compatibility = "compatible",
        controls = listOf(ChatGptWebUiControl(
            id = "temporary",
            semantic = "temporary_chat",
            label = "Temporary chat",
            region = ChatGptWebUiRegion.HEADER,
            role = "switch",
            enabled = true,
            selected = false,
            stateSettable = true,
        )),
    )
}
