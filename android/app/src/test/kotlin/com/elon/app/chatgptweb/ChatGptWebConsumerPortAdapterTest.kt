package com.elon.app.chatgptweb

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
        )
        val port = ChatGptWebConsumerPortAdapter(
            snapshot = { snapshot(streaming = true, dictationActive = false) },
            observedState = { observed },
            executeControl = { error("state reads must not use the MCP JSON port") },
        )

        val state = port.state()

        assertTrue(state.streaming)
        assertFalse(state.dictationActive)
        assertEquals("search", state.composerSections.getValue("tools").single().id)
        assertTrue(state.composerSections.getValue("tools").single().nativeSelector.contains("search"))
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
        val dictation = port.executeSessionCommand("chatgpt_start_dictation")
        val unsupported = port.executeSessionCommand("chatgpt_delete_account")

        assertEquals("chatgpt_list_composer_options", requests[0].getString("action"))
        assertEquals("tools", requests[0].getString("section"))
        assertEquals("search", requests[1].getString("option_id"))
        assertEquals("chatgpt_start_dictation", requests[2].getString("action"))
        assertTrue(requested.accepted)
        assertTrue(selected.accepted)
        assertEquals("mcp_1", dictation.requestId)
        assertFalse(unsupported.accepted)
        assertEquals("unsupported_consumer_command", unsupported.error)
        assertNull(unsupported.requestId)
    }

    private fun snapshot(
        streaming: Boolean,
        dictationActive: Boolean,
    ) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = "",
        messages = emptyList(),
        authenticated = false,
        composerReady = true,
        streaming = streaming,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = dictationActive,
        capabilities = ChatGptWebCapabilities.EMPTY,
    )
}
