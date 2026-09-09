package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebReadinessDispatchTest {
    private val page = ChatGptWebSnapshot(
        title = "Fixture", url = "https://chatgpt.com/", draft = "", messages = emptyList(),
        authenticated = true, composerReady = false, streaming = false, currentModel = "auto",
        attachments = emptyList(), dictationActive = false,
        capabilities = ChatGptWebCapabilities(emptySet()),
    )
    private val path = "/c/12345678-1234-1234-1234-123456789012"
    private val handle = "library_" + "a".repeat(32)
    private val download = "download_" + "b".repeat(32)

    private inner class Harness(current: Boolean = true) {
        var snapshot: ChatGptWebSnapshot? = page
        var bridge = ChatGptWebPageAdapter.State.CONNECTING
        var observed = ChatGptWebObservedState.Snapshot.EMPTY.copy(
            pageGeneration = 2, adapterGeneration = if (current) 2 else 1,
            conversations = listOf(ChatGptWebConversation("fixture", "Fixture", path, false)),
            composerSections = mapOf("model" to listOf(ChatGptWebComposerOption("old_option", "Fixture", false, "menuitem"))),
            features = listOf(ChatGptWebFeature("old_feature", "Fixture", "images", false)),
        )
        val calls = mutableListOf<String>()
        var receipts = 0
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onDispatch = { action, _ -> calls += action },
        ) {
            override fun listLibraryFiles(request: JSONObject, requestId: String) { calls += "library" }
            override fun cancelLibraryFiles(target: String, requestId: String) { calls += "cancel_library" }
            override fun listConversationFiles(path: String, requestId: String) { calls += "files" }
            override fun downloadLibraryFile(file: com.elon.app.WebChatLibraryEntry, requestId: String) { calls += "download" }
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { snapshot }, uiManifest = { null }, observedState = { observed },
            beginCommand = { action ->
                receipts++
                ChatGptWebObservedState.CommandRequest("mcp_test$receipts", action,
                    ChatGptWebObservedState.CommandRequest.PENDING, 1L)
            },
            bridgeState = { bridge }, mode = { ChatGptWebPresentationMode.NATIVE }, inputText = { "" },
            setInputText = {}, commands = commands, refresh = {}, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        val consumer = ChatGptWebConsumerPortAdapter({ snapshot }, { null }, { observed }, actions::control)
        fun control(action: String, configure: (JSONObject) -> Unit = {}): JSONObject =
            actions.control(JSONObject().put("action", action).also(configure))
    }

    @Test fun staleDocumentStillReturnsCachedConversationsButNotClickablePageHandles() {
        val h = Harness(current = false)
        h.snapshot = null
        val result = h.control("chatgpt_get_conversations")
        assertTrue(result.getBoolean("control_ok"))
        assertEquals(1, result.getJSONArray("conversations").length())
        assertTrue(result.getBoolean("stale"))
        assertFalse(result.getBoolean("adapter_current"))
        val navigation = h.control("chatgpt_get_navigation")
        assertTrue(navigation.getBoolean("control_ok"))
        assertEquals(0, navigation.getJSONArray("features").length())
        assertEquals(0, navigation.getJSONObject("composer_sections").length())
        assertEquals(0, h.receipts)
        assertTrue(h.calls.isEmpty())
    }

    @Test fun productionLibraryFilesAndModelQueriesDispatchWithoutComposerReadiness() {
        val h = Harness()
        assertTrue(h.consumer.requestLibraryFiles("", "", "open").accepted)
        assertTrue(h.consumer.requestConversationFiles(path).accepted)
        assertTrue(h.consumer.requestComposerOptions("model").accepted)
        assertEquals(listOf("library", "files", "list_model_options"), h.calls)
        assertEquals(3, h.receipts)
    }

    @Test fun unknownUiIdentityDispatchesToTheExistingPrivateIdentityOwner() {
        val h = Harness()
        h.snapshot = page.copy(authenticated = false)
        val result = h.control("chatgpt_list_library_files")
        assertTrue(result.getBoolean("control_ok"))
        assertEquals("dispatched", result.getString("command_status"))
        assertFalse(result.getBoolean("login_required"))
        assertEquals(1, h.receipts)
        assertEquals(listOf("library"), h.calls)
    }

    @Test fun nativeReadRequestsRejectOldGenerationWithoutSendingAnything() {
        val h = Harness(current = false)
        assertFalse(h.consumer.requestLibraryFiles("", "", "open").accepted)
        assertFalse(h.consumer.requestComposerOptions("model").accepted)
        assertEquals(0, h.receipts)
        assertTrue(h.calls.isEmpty())
    }

    @Test fun stopAndCancellationRemainReachableWhenTheComposerDisappears() {
        val h = Harness()
        assertTrue(h.control("chatgpt_stop_generation").getBoolean("control_ok"))
        assertTrue(h.control("chatgpt_cancel_library_files") {
            it.put("library_request_id", "mcp_previous")
        }.getBoolean("control_ok"))
        assertEquals(listOf("stop_generation", "cancel_library"), h.calls)
    }

    @Test fun accountDownloadStillRequiresTheExactObservedOpaqueHandle() {
        val h = Harness()
        val payload = JSONObject().put("version", 1).put("requestId", "mcp_files")
            .put("directoryHandle", "").put("query", "").put("breadcrumbs", JSONArray())
            .put("items", JSONArray().put(JSONObject().put("handle", handle).put("kind", "file")
                .put("name", "fixture.txt").put("mediaType", "text/plain").put("sizeBytes", 7)
                .put("downloadHandle", download)))
            .put("hasMore", false).put("partial", false).put("stale", false)
        h.observed = h.observed.copy(libraryFiles = ChatGptWebLibraryProtocol.parse(payload))
        assertFalse(h.consumer.downloadLibraryFile(handle, "download_" + "c".repeat(32)).accepted)
        assertEquals(0, h.receipts)
        assertTrue(h.consumer.downloadLibraryFile(handle, download).accepted)
        assertEquals(listOf("download"), h.calls)
    }

    @Test fun sendAndLibraryAttachmentCannotUseTheNewReadOnlyAdmission() {
        val h = Harness()
        assertEquals("bridge_not_ready", h.control("send_input").getString("error"))
        assertFalse(h.consumer.attachLibraryFile(handle).accepted)
        assertEquals(0, h.receipts)
        assertTrue(h.calls.isEmpty())
    }

    @Test fun currentContextCanBeReadButOldContextIsNotMistakenForTheCurrentConversation() {
        val h = Harness()
        assertTrue(h.control("chatgpt_get_context").getBoolean("control_ok"))
        h.observed = h.observed.copy(adapterGeneration = 1)
        assertEquals("adapter_generation_not_ready", h.control("chatgpt_get_context").getString("error"))
        assertEquals(0, h.receipts)
    }
}
