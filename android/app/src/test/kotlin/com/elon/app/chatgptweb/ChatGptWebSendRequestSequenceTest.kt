package com.elon.app.chatgptweb

import com.elon.app.PendingAttachment
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatSendAuthority
import com.elon.app.WebChatSendCommand
import com.elon.app.WebChatSendCoordinator
import com.elon.app.WebChatSendTransport
import com.elon.app.WebChatTransportDispatchResult
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSendRequestSequenceTest {
    @Test
    fun nativeMcpAndReadCommandsUseOneMonotonicSequenceBeyondTheOldCollision() {
        val fixture = Fixture()
        val ids = mutableListOf<String>()
        repeat(1200) { index ->
            ids += fixture.observed.beginCommand("list_conversations").id
            val requestId = if (index % 2 == 0) null else fixture.observed.beginCommand("send_prompt").id
            val sent = if (requestId == null) fixture.owner.dispatchSocial("fixture")
                else fixture.owner.dispatchMcp("fixture", requestId)
            assertEquals(WebChatSendCoordinator.DispatchOutcome.DISPATCHED, sent.outcome)
            val actual = fixture.commands.last().id
            ids += actual
            fixture.reject(actual)
        }
        assertEquals((1L..ids.size.toLong()).map { "mcp_${it.toString(36)}" }, ids)
        assertEquals(ids.size, ids.toSet().size)
        assertEquals(1200, fixture.commands.size)
    }

    @Test
    fun clearingOrRecreatingTheSendOwnerDoesNotRestartTheDocumentSequence() {
        val fixture = Fixture()
        val first = fixture.owner.dispatchSocial("first").commandId!!
        fixture.reject(first)
        fixture.owner.clear()
        fixture.observed.updateDocument(WebBridgeDocumentSession.Snapshot(2, 2, "doc_test_2"))
        val secondOwner = fixture.createOwner()
        assertEquals("mcp_1", first)
        assertEquals("mcp_2", secondOwner.dispatchSocial("second").commandId)
        assertEquals("mcp_3", fixture.observed.beginCommand("list_navigation").id)
    }

    @Test
    fun attachmentReservationsShareTheSequenceWithoutDispatchingTextEarly() {
        val fixture = Fixture()
        val file = PendingAttachment(
            kind = "file", displayName = "note.txt", fileName = "note.txt",
            mimeType = "text/plain", file = File("note.txt"),
        )
        assertTrue(fixture.owner.beginAttachments("file fixture", listOf(file)))
        assertEquals(listOf("mcp_1"), fixture.uploadIds)
        assertTrue(fixture.commands.isEmpty())
        assertEquals("mcp_2", fixture.observed.beginCommand("list_navigation").id)
        fixture.owner.clear()
        assertEquals("mcp_3", fixture.owner.dispatchSocial("next").commandId)
    }

    @Test
    fun busyAndNotReadyClicksDoNotConsumeOrRegisterCommands() {
        val fixture = Fixture()
        assertEquals("mcp_1", fixture.owner.dispatchSocial("first").commandId)
        assertEquals(WebChatSendCoordinator.DispatchOutcome.BUSY,
            fixture.owner.dispatchSocial("busy").outcome)
        assertEquals("mcp_2", fixture.observed.nextRequestId())
        fixture.reject("mcp_1")
        fixture.ready = false
        assertEquals(WebChatSendCoordinator.DispatchOutcome.NOT_READY,
            fixture.owner.dispatchSocial("not ready").outcome)
        assertEquals("mcp_3", fixture.observed.nextRequestId())
        assertTrue(fixture.observed.snapshot().commandRequests.isEmpty())
    }

    private class Fixture {
        val observed = ChatGptWebObservedState()
        val commands = mutableListOf<WebChatSendCommand>()
        val uploadIds = mutableListOf<String>()
        var ready = true
        val owner = createOwner()

        fun createOwner() = ChatGptWebSendOwner(
            nextRequestId = observed::nextRequestId,
            transport = object : WebChatSendTransport {
                override val authority = WebChatSendAuthority.OFFICIAL_PAGE
                override fun isReady() = ready
                override fun dispatch(command: WebChatSendCommand): WebChatTransportDispatchResult {
                    commands += command
                    return WebChatTransportDispatchResult.QUEUED
                }
                override fun reconcile() = Unit
            },
            snapshot = { snapshot() },
            stageUploads = { emptyList() },
            requestAttachmentUpload = { error("unexpected_compatibility_upload") },
            removeAttachment = {},
            postDelayed = { _, _ -> },
            removeCallbacks = {},
            onTerminalTimeout = {},
            onAttachmentChanged = {},
            onSendStateChanged = {},
            requestPrivateAttachmentUpload = { _, _, id -> uploadIds += id; true },
        )

        fun reject(id: String) {
            owner.acceptCommandResult(ChatGptWebEvent.CommandResult("send_prompt", false, "", id))
            assertNull(owner.prompt())
        }
    }

    private companion object {
        fun snapshot() = ChatGptWebSnapshot(
            title = "", url = "https://chatgpt.com/c/11111111-1111-4111-8111-111111111111",
            draft = "", messages = emptyList(), authenticated = true, composerReady = true,
            streaming = false, currentModel = "", attachments = emptyList(), dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY,
        )
    }
}
