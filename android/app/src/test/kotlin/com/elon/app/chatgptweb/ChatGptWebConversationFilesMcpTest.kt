package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebConversationFilesMcpTest {
    @Test fun runtimeInventoryDoesNotPretendTheIndexIsDeviceVerifiedOrAnUploader() {
        val rows = WebAiPrivateTransportCatalog.describe()
        val row = (0 until rows.length()).map(rows::getJSONObject).single {
            it.getString("capability_id") == "android_chatgpt_private_conversation_files_v1"
        }
        assertEquals("implemented_device_pending", row.getString("implementation_status"))
        assertEquals("same_origin_history_get_to_native_attachment_index", row.getString("request_mode"))
        assertTrue(row.getBoolean("production_default"))
        assertFalse(row.getBoolean("direct_post_enabled"))
    }

    @Test fun productionConsumerUsesTrackedFileCommandNotNavigationOrDraftMutation() {
        val state = ChatGptWebObservedState()
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_test"))
        var calledPath = ""
        var calledRequest = ""
        var downloadPath = ""
        var downloadRequest = ""
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort(
            onOpenConversation = { fail("file listing must not navigate") },
        ) {
            override fun listConversationFiles(path: String, requestId: String) {
                calledPath = path
                calledRequest = requestId
            }
            override fun downloadConversationFile(path: String, file: com.elon.app.WebChatConversationFile, requestId: String) {
                downloadPath = path
                downloadRequest = requestId
                assertEquals("download_" + "a".repeat(32), file.downloadHandle)
            }
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { null }, uiManifest = { null }, observedState = state::snapshot,
            beginCommand = state::beginCommand, beginConversationFilesCommand = state::beginConversationFilesCommand,
            bridgeState = { ChatGptWebPageAdapter.State.READY }, mode = { ChatGptWebPresentationMode.NATIVE },
            inputText = { "unsent draft" }, setInputText = { fail("must not change draft") },
            commands = commands, refresh = { fail("must not reload") }, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        val consumer = ChatGptWebConsumerPortAdapter({ null }, { null }, state::snapshot, actions::control)
        val result = consumer.requestConversationFiles("/g/g-p-demo/c/fixture")
        assertTrue(result.accepted)
        assertEquals("/g/g-p-demo/c/fixture", calledPath)
        assertEquals(result.requestId, calledRequest)
        assertEquals(calledPath, state.snapshot().commandRequests.single().targetConversationPath)
        assertFalse(consumer.requestConversationFiles("https://example.com/c/wrong").accepted)
        assertEquals(1, state.snapshot().commandRequests.size)
        assertNull(consumer.conversationFiles("/c/unknown"))
        val fixture = JSONObject(requireNotNull(javaClass.classLoader?.getResourceAsStream(
            "webchat/private-conversation-files-contract.json")).bufferedReader().use { it.readText() }).getJSONObject("event")
        fixture.getJSONArray("files").getJSONObject(0).put("downloadHandle", "download_" + "a".repeat(32))
        state.accept(ChatGptWebEvent.ConversationFiles(requireNotNull(ChatGptWebConversationFiles.parse(
            fixture.put("requestId", result.requestId)))))
        assertEquals(2, consumer.conversationFiles(calledPath)?.files?.size)
        assertEquals(consumer.conversationFiles(calledPath), consumer.conversationFiles("/c/fixture"))
        val file = requireNotNull(consumer.conversationFiles(calledPath)).files.first()
        assertFalse(consumer.downloadConversationFile(calledPath, file.id, "download_" + "b".repeat(32)).accepted)
        assertEquals("", downloadRequest)
        val requested = consumer.downloadConversationFile("/c/fixture", file.id, file.downloadHandle)
        assertTrue(requested.accepted)
        assertEquals("/c/fixture", downloadPath)
        assertEquals(requested.requestId, downloadRequest)
        state.accept(ChatGptWebEvent.CommandResult("download_conversation_file", true, "download_queued", requested.requestId))
        assertEquals(com.elon.app.WebChatConsumerCommandStatus.SUCCEEDED,
            consumer.state().commandRequests.single { it.id == requested.requestId }.status)
        state.accept(ChatGptWebEvent.CommandResult("request_attachment_upload", true,
            "private_attachment_associated", "mcp_s1"))
        state.accept(ChatGptWebEvent.CommandResult("send_prompt", true, "sent", "mcp_s1"))
        val ui = actions.uiState()
        assertEquals("send_prompt", ui.getJSONObject("last_command").getString("action"))
        assertEquals("private_attachment_associated",
            ui.getJSONObject("last_attachment_upload").getString("detail"))
        assertTrue(ui.isNull("conversation_files"))
        state.updateDocument(WebBridgeDocumentSession.Snapshot(2, 2, "next_document"))
        assertTrue(actions.uiState().isNull("last_attachment_upload"))
    }

    @Test fun currentFileIndexNeedsCurrentDocumentRouteAndMatchingCompletedReceipt() {
        val state = ChatGptWebObservedState(nowMs = { 1_000L })
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_test"))
        val path = "/c/fixture"
        val request = state.beginConversationFilesCommand(path)
        val file = com.elon.app.WebChatConversationFile(
            "message:0", "message", "fixture.txt", "file", "user", "text/plain",
            "download_" + "a".repeat(32),
        )
        state.accept(ChatGptWebEvent.ConversationFiles(com.elon.app.WebChatConversationFileIndex(
            path, request.id, listOf(file), truncated = false,
        )))
        fun result(url: String = "https://chatgpt.com/c/fixture", now: Long = 1_000L) =
            ChatGptWebMcpSnapshotJson.conversationFiles(state.snapshot(), url, now)
        assertSame(JSONObject.NULL, result())
        state.accept(ChatGptWebEvent.CommandResult("list_conversation_files", true, "", request.id))
        val current = result() as JSONObject
        assertEquals(path, current.getString("conversation_path"))
        assertEquals(request.id, current.getString("request_id"))
        assertFalse(current.getBoolean("stale"))
        assertEquals(file.downloadHandle, current.getJSONArray("files").getJSONObject(0).getString("download_handle"))
        assertEquals(file.id, current.getJSONArray("files").getJSONObject(0).getString("file_id"))
        assertSame(JSONObject.NULL, result("https://chatgpt.com/c/other"))
        assertSame(JSONObject.NULL, result("https://other.example/c/fixture"))
        assertSame(JSONObject.NULL, result("https://chatgpt.com/"))
        val stale = result(now = 61_000L) as JSONObject
        assertTrue(stale.getBoolean("stale"))
        assertEquals("", stale.getJSONArray("files").getJSONObject(0).getString("download_handle"))
        state.updateDocument(WebBridgeDocumentSession.Snapshot(2, 2, "next_document"))
        assertSame(JSONObject.NULL, result())
    }

    @Test fun currentFileIndexNeverExportsUnsafeDownloadHandlesOrAnotherCachedIndex() {
        val file = com.elon.app.WebChatConversationFile(
            "message:0", "message", "fixture.txt", "file", "user", "text/plain",
            "https://blob.example.invalid/?secret=not-a-handle",
        )
        val state = ChatGptWebObservedState(nowMs = { 1_000L })
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_test"))
        for (path in listOf("/c/other", "/c/fixture")) {
            val request = state.beginConversationFilesCommand(path)
            state.accept(ChatGptWebEvent.ConversationFiles(com.elon.app.WebChatConversationFileIndex(
                path, request.id, listOf(file), truncated = false,
            )))
            state.accept(ChatGptWebEvent.CommandResult("list_conversation_files", true, "", request.id))
        }
        val result = ChatGptWebMcpSnapshotJson.conversationFiles(
            state.snapshot(), "https://chatgpt.com/c/fixture", 1_000L,
        ) as JSONObject
        assertEquals(1, result.getJSONArray("files").length())
        assertEquals("", result.getJSONArray("files").getJSONObject(0).getString("download_handle"))
        assertFalse(result.toString().contains("secret"))
        assertFalse(result.toString().contains("/c/other"))
        assertSame(JSONObject.NULL, ChatGptWebMcpSnapshotJson.conversationFiles(
            state.snapshot().copy(adapterGeneration = 0), "https://chatgpt.com/c/fixture", 1_000L,
        ))
    }
}
