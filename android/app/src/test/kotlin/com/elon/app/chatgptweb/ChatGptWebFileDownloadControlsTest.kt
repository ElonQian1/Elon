package com.elon.app.chatgptweb

import com.elon.app.WebChatFileDownloadState
import com.elon.app.WebChatFileDownloadState.Stage
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebFileDownloadControlsTest {
    @Test fun consumerAndMcpCancelOnlyTheRequestedTransferEvenWhenWebBridgeIsNotReady() {
        val observed = ChatGptWebObservedState()
        val session = ChatGptWebFileDownloadSession()
        session.begin("private-lease-not-exported", "download-request")
        session.update("private-lease-not-exported", Stage.TRANSFERRING, 12, 24)
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun fileDownloadState(): WebChatFileDownloadState? = session.snapshot()
            override fun cancelFileDownload(requestId: String): Boolean = session.requestCancel(requestId) != null
        }
        val actions = ChatGptWebMcpActions(
            snapshot = { null }, uiManifest = { null }, observedState = observed::snapshot,
            beginCommand = observed::beginCommand, bridgeState = { ChatGptWebPageAdapter.State.CONNECTING },
            mode = { ChatGptWebPresentationMode.NATIVE }, inputText = { "" }, setInputText = { fail("must not change draft") },
            commands = commands, refresh = { fail("must not reload") }, selectMode = {}, revealMessage = { _, _, _ -> false },
        )
        val consumer = ChatGptWebConsumerPortAdapter({ null }, { null }, observed::snapshot, actions::control, session::snapshot)
        assertEquals(12L, consumer.fileDownloadState()?.receivedBytes)
        val json = actions.uiState().getJSONObject("file_download")
        assertEquals(setOf("request_id", "state", "received_bytes", "total_bytes", "can_cancel"), json.keys().asSequence().toSet())
        assertFalse(json.toString().contains("private-lease"))
        assertFalse(consumer.cancelFileDownload("old-request").accepted)
        assertEquals(Stage.TRANSFERRING, session.snapshot()?.stage)
        assertTrue(consumer.cancelFileDownload("download-request").accepted)
        assertEquals(Stage.CANCELLING, session.snapshot()?.stage)
        session.update("private-lease-not-exported", Stage.CANCELLED)
        assertFalse(actions.control(JSONObject().put("action", "chatgpt_cancel_file_download")
            .put("download_request_id", "download-request")).optBoolean("control_ok"))
        assertTrue(ChatGptWebMcpActionCatalog.availableActions.contains("chatgpt_cancel_file_download"))
    }
}
