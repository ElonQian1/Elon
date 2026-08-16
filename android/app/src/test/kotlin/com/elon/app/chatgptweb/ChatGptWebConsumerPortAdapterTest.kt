package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerCommandStatus
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
                    pageKind = "health",
                    url = "https://chatgpt.com/health",
                )
            },
            observedState = { observed },
            executeControl = { error("state reads must not use the MCP JSON port") },
        )

        val state = port.state()

        assertTrue(state.streaming)
        assertFalse(state.dictationActive)
        assertEquals("health", state.pageKind)
        assertEquals("https://chatgpt.com/health", state.pageUrl)
        assertEquals("search", state.composerSections.getValue("tools").single().id)
        assertTrue(state.composerSections.getValue("tools").single().nativeSelector.contains("search"))
        assertTrue(state.features.single().requiresUserConfirmation)
        assertTrue(state.features.single().nativeSelector.contains("health"))
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
            observedState = { observed },
            executeControl = { JSONObject() },
        )

        val state = port.state()

        assertFalse(state.streaming)
        assertFalse(state.dictationActive)
        assertTrue(state.composerSections.isEmpty())
        assertTrue(state.features.isEmpty())
        assertEquals("unknown", state.pageKind)
        assertEquals("", state.pageUrl)
    }

    @Test
    fun mapsConsumerCommandsToTheExistingControlExecutor() {
        val requests = mutableListOf<JSONObject>()
        val port = ChatGptWebConsumerPortAdapter(
            snapshot = { null },
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
        val dictation = port.executeSessionCommand("chatgpt_start_dictation")
        val unsupported = port.executeSessionCommand("chatgpt_delete_account")

        assertEquals("chatgpt_list_composer_options", requests[0].getString("action"))
        assertEquals("tools", requests[0].getString("section"))
        assertEquals("search", requests[1].getString("option_id"))
        assertEquals("chatgpt_list_features", requests[2].getString("action"))
        assertEquals("health", requests[3].getString("feature_id"))
        assertTrue(requests[3].getBoolean("user_confirmed"))
        assertEquals("chatgpt_start_dictation", requests[4].getString("action"))
        assertTrue(requested.accepted)
        assertTrue(selected.accepted)
        assertTrue(features.accepted)
        assertTrue(feature.accepted)
        assertEquals("mcp_1", dictation.requestId)
        assertFalse(unsupported.accepted)
        assertEquals("unsupported_consumer_command", unsupported.error)
        assertNull(unsupported.requestId)
    }

    private fun snapshot(
        streaming: Boolean,
        dictationActive: Boolean,
        pageKind: String = "unknown",
        url: String = "https://chatgpt.com/",
    ) = ChatGptWebSnapshot(
        title = "",
        url = url,
        draft = "",
        messages = emptyList(),
        authenticated = false,
        composerReady = true,
        streaming = streaming,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = dictationActive,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = pageKind,
    )
}
