package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMcpGenerationTest {
    @Test
    fun exposesCurrentDocumentGenerationInUiState() {
        val state = actions(pageGeneration = 7, adapterGeneration = 7).uiState()

        assertEquals(7L, state.getLong("page_generation"))
        assertEquals(7L, state.getLong("adapter_generation"))
        assertTrue(state.getBoolean("adapter_current"))
        assertTrue(state.getBoolean("authenticated"))
    }

    @Test
    fun rejectsOfficialCommandsFromAnOldAdapterGeneration() {
        var dispatchCount = 0
        val result = actions(
            pageGeneration = 8,
            adapterGeneration = 7,
            onRequestComposerOptions = { dispatchCount++ },
        ).control(JSONObject()
            .put("action", "chatgpt_list_composer_options")
            .put("section", "model"))

        assertFalse(result.getBoolean("control_ok"))
        assertEquals("adapter_generation_not_ready", result.getString("error"))
        assertEquals(0, dispatchCount)
        assertFalse(result.getBoolean("authenticated"))
        assertTrue(result.isNull("ui_manifest"))
    }

    @Test
    fun refreshIsLocalAndRequiresANewerAdapterGenerationForCompletion() {
        var refreshCount = 0
        val result = actions(
            pageGeneration = 9,
            adapterGeneration = 0,
            bridgeState = ChatGptWebPageAdapter.State.CONNECTING,
            onRefresh = { refreshCount++ },
        ).control(JSONObject().put("action", "chatgpt_refresh"))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals(1, refreshCount)
        assertEquals(9L, result.getLong("refresh_from_page_generation"))
        assertEquals("adapter_generation", result.getString("completion_signal"))
        assertTrue(result.getString("poll_hint").contains("adapter_current=true"))
    }

    @Test
    fun nativeMessageRevealValidatesCurrentIdsTargetsAndStructuredPartIndexes() {
        var revealed: Triple<String, Int?, String>? = null
        val actions = actions(
            pageGeneration = 1,
            adapterGeneration = 1,
            snapshot = MESSAGE_SNAPSHOT,
            onRevealMessage = { messageId, partIndex, nativeTarget ->
                revealed = Triple(messageId, partIndex, nativeTarget)
                true
            },
        )

        val result = actions.control(JSONObject()
              .put("action", "chatgpt_reveal_message")
              .put("message_id", "message_demo")
              .put("target", "actions"))
        assertTrue(result.getBoolean("control_ok"))
        assertEquals(Triple("message_demo", null, "actions"), revealed)

        val part = actions.control(JSONObject()
            .put("action", "chatgpt_reveal_message")
            .put("message_id", "message_demo")
            .put("part_index", 0))
        assertTrue(part.getBoolean("control_ok"))
        assertEquals(Triple("message_demo", 0, "message"), revealed)

        val stale = actions.control(JSONObject()
            .put("action", "chatgpt_reveal_message")
            .put("message_id", "missing"))
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("stale_message_id", stale.getString("error"))

        val invalidPart = actions.control(JSONObject()
            .put("action", "chatgpt_reveal_message")
            .put("message_id", "message_demo")
            .put("part_index", 2))
        assertFalse(invalidPart.getBoolean("control_ok"))
        assertEquals("invalid_part_index", invalidPart.getString("error"))

        val invalidTarget = actions.control(JSONObject()
            .put("action", "chatgpt_reveal_message")
            .put("message_id", "message_demo")
            .put("target", "unknown"))
        assertFalse(invalidTarget.getBoolean("control_ok"))
        assertEquals("invalid_reveal_target", invalidTarget.getString("error"))

        val conflictingTarget = actions.control(JSONObject()
            .put("action", "chatgpt_reveal_message")
            .put("message_id", "message_demo")
            .put("part_index", 0)
            .put("target", "actions"))
        assertFalse(conflictingTarget.getBoolean("control_ok"))
        assertEquals("part_target_conflict", conflictingTarget.getString("error"))
    }

    private fun actions(
        pageGeneration: Long,
        adapterGeneration: Long,
        bridgeState: ChatGptWebPageAdapter.State = ChatGptWebPageAdapter.State.READY,
        snapshot: ChatGptWebSnapshot = SNAPSHOT,
        onRefresh: () -> Unit = {},
        onRequestComposerOptions: () -> Unit = {},
        onRevealMessage: (String, Int?, String) -> Boolean = { _, _, _ -> false },
    ) = ChatGptWebMcpActions(
        snapshot = { snapshot },
        uiManifest = { MANIFEST },
        observedState = {
            ChatGptWebObservedState.Snapshot.EMPTY.copy(
                pageGeneration = pageGeneration,
                adapterGeneration = adapterGeneration,
            )
        },
        beginCommand = { expectedAction ->
            ChatGptWebObservedState.CommandRequest(
                id = "mcp_generation",
                expectedAction = expectedAction,
                status = ChatGptWebObservedState.CommandRequest.PENDING,
                startedAtMs = 1L,
            )
        },
        bridgeState = { bridgeState },
        mode = { ChatGptWebPresentationMode.NATIVE },
        inputText = { "" },
        setInputText = {},
        commands = object : ChatGptWebMcpCommandPort {
            override fun setDraft(value: String, expectedDraft: String, requestId: String) = Unit
            override fun sendInput(requestId: String) = Unit
            override fun invokeControl(controlId: String, requestId: String) = Unit
            override fun setControlText(controlId: String, text: String, requestId: String) = Unit
            override fun setControlSelected(controlId: String, selected: Boolean, requestId: String) = Unit
            override fun selectControlChoice(controlId: String, choiceIndex: Int, requestId: String) = Unit
            override fun setControlSlider(controlId: String, value: Double, requestId: String) = Unit
            override fun setControlExpanded(controlId: String, expanded: Boolean, requestId: String) = Unit
            override fun newConversation(requestId: String) = Unit
            override fun stopGeneration(requestId: String) = Unit
            override fun verifyPrivateStreamWatchdog(requestId: String) = Unit
            override fun regenerateResponse(requestId: String) = Unit
            override fun startDictation(requestId: String) = Unit
            override fun cancelDictation(requestId: String) = Unit
            override fun submitDictation(requestId: String) = Unit
            override fun removeAttachment(attachmentId: String, requestId: String) = Unit
            override fun refreshControls(requestId: String) = Unit
            override fun listConversations(requestId: String) = Unit
            override fun requestComposerOptions(section: String, requestId: String) {
                onRequestComposerOptions()
            }
            override fun dismissComposerOptions(requestId: String) = Unit
            override fun selectComposerOption(section: String, optionId: String, requestId: String) = Unit
            override fun requestFeatures(requestId: String) = Unit
            override fun dismissFeatures(requestId: String) = Unit
            override fun selectFeature(featureId: String, requestId: String) = Unit
            override fun openConversation(path: String, requestId: String) = Unit
        },
        refresh = onRefresh,
        selectMode = {},
        revealMessage = onRevealMessage,
    )

    private companion object {
        val SNAPSHOT = ChatGptWebSnapshot(
            title = "Work",
            url = "https://chatgpt.com/",
            draft = "",
            messages = emptyList(),
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "model",
            attachments = emptyList(),
            dictationActive = false,
            capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.MODEL_SELECTOR)),
        )
        val MESSAGE_SNAPSHOT = SNAPSHOT.copy(
            messages = listOf(
                ChatGptWebMessage(
                    id = "message_demo",
                    role = "assistant",
                    content = "demo",
                    state = "completed",
                    parts = listOf(ChatGptWebMessagePart("file", "demo.txt")),
                ),
            ),
            pageKind = "conversation",
            observedMessageCount = 1,
        )
        val MANIFEST = ChatGptWebUiManifest(
            version = 3,
            pageKind = "home",
            title = "Work",
            compatibility = "healthy",
            controls = emptyList(),
        )
    }
}
