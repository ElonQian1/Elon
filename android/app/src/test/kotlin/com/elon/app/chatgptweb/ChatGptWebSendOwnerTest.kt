package com.elon.app.chatgptweb

import com.elon.app.PendingAttachment
import com.elon.app.WebChatPendingSendState
import com.elon.app.WebChatSendAuthority
import com.elon.app.WebChatSendCommand
import com.elon.app.WebChatSendCoordinator
import com.elon.app.WebChatSendTransport
import com.elon.app.WebChatTransportDispatchResult
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSendOwnerTest {
    @Test
    fun mcpRequestIdIsOwnedByTheSameLedgerAsSocialSends() {
        val fixture = Fixture()

        val mcp = fixture.owner.dispatchMcp("from mcp", "mcp_external1")
        val competing = fixture.owner.dispatchSocial("from ui")

        assertEquals(WebChatSendCoordinator.DispatchOutcome.DISPATCHED, mcp.outcome)
        assertEquals(WebChatSendCoordinator.DispatchOutcome.BUSY, competing.outcome)
        assertEquals("mcp_external1", fixture.transport.commands.single().id)

        val receipt = fixture.owner.acceptCommandResult(
            commandResult("send_prompt", ok = true, requestId = "mcp_external1"),
        )

        assertEquals(ChatGptWebSendOrigin.MCP, receipt?.origin)
        assertTrue(receipt?.ok == true)
        assertEquals("from mcp", fixture.owner.prompt())
    }

    @Test
    fun attachmentUploadReservesTheOnlySendSlotUntilItsPromptIsDispatched() {
        val fixture = Fixture()

        assertTrue(fixture.owner.beginAttachments("with file", listOf(pendingAttachment())))
        assertEquals(1, fixture.attachmentUploadRequests)
        assertEquals("uploading", fixture.owner.attachmentSendPhase())
        assertEquals(
            WebChatSendCoordinator.DispatchOutcome.BUSY,
            fixture.owner.dispatchMcp("competing", "mcp_competing").outcome,
        )
        assertTrue(fixture.transport.commands.isEmpty())

        fixture.currentSnapshot = snapshot(
            attachments = listOf(ChatGptWebAttachment("upload-1", "note.txt", "ready", true)),
        )
        fixture.owner.observeSnapshot(fixture.currentSnapshot)

        val command = fixture.transport.commands.single()
        assertEquals("with file", command.prompt)
        assertEquals("sending", fixture.owner.attachmentSendPhase())
        assertEquals(
            ChatGptWebSendOrigin.ATTACHMENT,
            fixture.owner.acceptCommandResult(
                commandResult("send_prompt", ok = true, requestId = command.id),
            )?.origin,
        )

        fixture.currentSnapshot = snapshot(
            messages = listOf(message("user-1", "user", "with file")),
            attachments = fixture.currentSnapshot.attachments,
        )
        fixture.owner.observeSnapshot(fixture.currentSnapshot)

        assertEquals("completed", fixture.owner.attachmentSendPhase())
        assertEquals("completed", fixture.attachmentUpdates.last().phase)
        assertEquals("with file", fixture.owner.prompt())

        fixture.currentSnapshot = snapshot(
            messages = listOf(
                message("user-1", "user", "with file"),
                message("assistant-1", "assistant", "done"),
            ),
            attachments = fixture.currentSnapshot.attachments,
        )
        fixture.owner.observeSnapshot(fixture.currentSnapshot)

        assertNull(fixture.owner.prompt())
    }

    @Test
    fun uploadFailureBeforePromptDispatchReleasesTheReservation() {
        val fixture = Fixture()
        assertTrue(fixture.owner.beginAttachments("failed file", listOf(pendingAttachment())))

        fixture.owner.acceptCommandResult(
            commandResult("request_attachment_upload", ok = false),
        )
        assertEquals("failed", fixture.owner.attachmentSendPhase())
        val next = fixture.owner.dispatchSocial("next message")

        assertEquals("idle", fixture.owner.attachmentSendPhase())
        assertEquals(WebChatSendCoordinator.DispatchOutcome.DISPATCHED, next.outcome)
        assertEquals(listOf("next message"), fixture.transport.commands.map(WebChatSendCommand::prompt))
    }

    @Test
    fun localStagingFailureReleasesTheReservationWithoutDispatching() {
        val fixture = Fixture(stageSucceeds = false)

        assertFalse(fixture.owner.beginAttachments("failed staging", listOf(pendingAttachment())))
        val next = fixture.owner.dispatchSocial("next message")

        assertEquals(WebChatSendCoordinator.DispatchOutcome.DISPATCHED, next.outcome)
        assertEquals(listOf("next message"), fixture.transport.commands.map(WebChatSendCommand::prompt))
        assertTrue(fixture.attachmentUpdates.isEmpty())
    }

    private class Fixture(stageSucceeds: Boolean = true) {
        val transport = FakeTransport()
        val scheduler = FakeScheduler()
        val attachmentUpdates = mutableListOf<ChatGptWebAttachmentSendUpdate>()
        val terminalTimeouts = mutableListOf<WebChatPendingSendState.TimeoutResult>()
        var attachmentUploadRequests = 0
        var currentSnapshot = snapshot()
        val owner = ChatGptWebSendOwner(
            transport = transport,
            snapshot = { currentSnapshot },
            stageUploads = { if (stageSucceeds) emptyList() else null },
            requestAttachmentUpload = {
                attachmentUploadRequests += 1
                true
            },
            removeAttachment = {},
            postDelayed = scheduler::postDelayed,
            removeCallbacks = scheduler::removeCallbacks,
            onTerminalTimeout = terminalTimeouts::add,
            onAttachmentChanged = attachmentUpdates::add,
            onSendStateChanged = {},
            confirmationTimeoutMs = 10L,
            attachmentTimeoutMs = 100L,
        )
    }

    private class FakeTransport : WebChatSendTransport {
        override val authority = WebChatSendAuthority.OFFICIAL_PAGE
        val commands = mutableListOf<WebChatSendCommand>()

        override fun isReady(): Boolean = true

        override fun dispatch(command: WebChatSendCommand): WebChatTransportDispatchResult {
            commands += command
            return WebChatTransportDispatchResult.QUEUED
        }

        override fun reconcile() = Unit
    }

    private class FakeScheduler {
        private val tasks = linkedSetOf<Runnable>()

        fun postDelayed(task: Runnable, @Suppress("UNUSED_PARAMETER") delayMs: Long) {
            tasks += task
        }

        fun removeCallbacks(task: Runnable) {
            tasks -= task
        }
    }

    private companion object {
        fun snapshot(
            messages: List<ChatGptWebMessage> = emptyList(),
            attachments: List<ChatGptWebAttachment> = emptyList(),
        ) = ChatGptWebSnapshot(
            title = "",
            url = "https://chatgpt.com/c/test",
            draft = "",
            messages = messages,
            authenticated = true,
            composerReady = true,
            streaming = false,
            currentModel = "",
            attachments = attachments,
            dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY,
        )

        fun message(id: String, role: String, content: String) = ChatGptWebMessage(
            id = id,
            role = role,
            content = content,
            state = "complete",
            parts = emptyList(),
        )

        fun pendingAttachment() = PendingAttachment(
            kind = "file",
            displayName = "note.txt",
            fileName = "note.txt",
            mimeType = "text/plain",
            file = File("note.txt"),
        )

        fun commandResult(action: String, ok: Boolean, requestId: String? = null) =
            ChatGptWebEvent.CommandResult(action, ok, "", requestId)
    }
}
