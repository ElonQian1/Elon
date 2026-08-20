package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptRealtimeVoicePreparationTest {
    @Test
    fun clearsOnlyTheObservedStaleOfficialDraft() {
        var value = "not called"
        var expectedDraft = ""
        var dispatched = ""
        val actions = actions(
            officialDraft = "stale official draft",
            onSetDraft = { next, expected ->
                value = next
                expectedDraft = expected
            },
            onDispatch = { action -> dispatched = action },
        )

        val result = actions.control(JSONObject().put("action", PREPARE_ACTION))

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("", value)
        assertEquals("stale official draft", expectedDraft)
        assertEquals("set_draft", dispatched)
        assertEquals(
            "set_draft",
            result.getJSONObject("command_receipt").getString("expected_web_action"),
        )
    }

    @Test
    fun neverClearsWhileTheNativeComposerHasText() {
        var setDraftCount = 0
        val actions = actions(
            officialDraft = "official draft",
            nativeDraft = "native draft",
            onSetDraft = { _, _ -> setDraftCount += 1 },
        )

        val result = actions.control(JSONObject().put("action", PREPARE_ACTION))

        assertFalse(result.getBoolean("control_ok"))
        assertEquals("native_draft_not_empty", result.getString("error"))
        assertEquals(0, setDraftCount)
    }

    @Test
    fun skipsDraftMutationWhenBothComposersAreAlreadyEmpty() {
        var setDraftCount = 0
        val result = actions(
            officialDraft = "",
            onSetDraft = { _, _ -> setDraftCount += 1 },
        ).control(JSONObject().put("action", PREPARE_ACTION))

        assertTrue(result.getBoolean("control_ok"))
        assertFalse(result.has("command_receipt"))
        assertEquals(0, setDraftCount)
    }

    private fun actions(
        officialDraft: String,
        nativeDraft: String = "",
        onSetDraft: (String, String) -> Unit,
        onDispatch: (String) -> Unit = {},
    ): ChatGptWebMcpActions {
        var nextCommandId = 0
        return ChatGptWebMcpActions(
            snapshot = { snapshot(officialDraft) },
            uiManifest = { manifest() },
            observedState = {
                ChatGptWebObservedState.Snapshot.EMPTY.copy(
                    pageGeneration = 1L,
                    adapterGeneration = 1L,
                )
            },
            beginCommand = { expectedAction ->
                ChatGptWebObservedState.CommandRequest(
                    id = "voice_${++nextCommandId}",
                    expectedAction = expectedAction,
                    status = ChatGptWebObservedState.CommandRequest.PENDING,
                    startedAtMs = 1L,
                )
            },
            bridgeState = { ChatGptWebPageAdapter.State.READY },
            mode = { ChatGptWebPresentationMode.NATIVE },
            inputText = { nativeDraft },
            setInputText = {},
            commands = ChatGptWebMcpTestCommandPort(
                onSetDraft = onSetDraft,
                onDispatch = { action, _ -> onDispatch(action) },
            ),
            refresh = {},
            selectMode = {},
            revealMessage = { _, _, _ -> false },
        )
    }

    private fun snapshot(draft: String) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = draft,
        messages = emptyList(),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "conversation",
    )

    private fun manifest() = ChatGptWebUiManifest(
        version = 1,
        pageKind = "conversation",
        title = "",
        compatibility = "healthy",
        controls = emptyList(),
    )

    private companion object {
        const val PREPARE_ACTION = "chatgpt_prepare_realtime_voice"
    }
}
