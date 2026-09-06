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
    }
}
