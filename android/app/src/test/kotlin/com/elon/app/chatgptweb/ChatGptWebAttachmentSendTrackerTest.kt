package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAttachmentSendTrackerTest {
    @Test
    fun waitsForEveryNewAttachmentBeforeSendingPrompt() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 2, snapshot())

        assertTrue(tracker.observe(snapshot(attachments = listOf(attachment("a", "uploading")))) is
            ChatGptWebAttachmentSendTracker.Observation.Wait)
        assertTrue(tracker.observe(snapshot(attachments = listOf(
            attachment("a", "ready"),
            attachment("b", "ready"),
        ))) is ChatGptWebAttachmentSendTracker.Observation.SendPrompt)
        assertEquals(ChatGptWebAttachmentSendTracker.Phase.SENDING, tracker.phase)
    }

    @Test
    fun doesNotSendWhenAnAttachmentFailed() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 1, snapshot())

        val result = tracker.observe(snapshot(attachments = listOf(attachment("a", "error"))))

        assertTrue(result is ChatGptWebAttachmentSendTracker.Observation.Failed)
        assertEquals(ChatGptWebAttachmentSendTracker.Phase.FAILED, tracker.phase)
    }

    @Test
    fun completesOnlyAfterTheExpectedNewUserMessageAppears() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 1, snapshot())
        tracker.observe(snapshot(attachments = listOf(attachment("a", "ready"))))

        assertTrue(tracker.observe(snapshot(messages = listOf(message("u-other", "user", "其他消息")))) is
            ChatGptWebAttachmentSendTracker.Observation.Wait)
        val completed = tracker.observe(snapshot(messages = listOf(message("u-new", "user", "分析附件"))))

        assertTrue(completed is ChatGptWebAttachmentSendTracker.Observation.Complete)
        assertEquals("u-new", (completed as ChatGptWebAttachmentSendTracker.Observation.Complete).userMessageId)
    }

    @Test
    fun attachmentOnlySendRequiresStructuredAttachmentEvidence() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("", 1, snapshot())
        tracker.observe(snapshot(attachments = listOf(attachment("a", "ready"))))

        assertTrue(tracker.observe(snapshot(messages = listOf(message("u-text", "user", "")))) is
            ChatGptWebAttachmentSendTracker.Observation.Wait)
        val message = message("u-file", "user", "", partType = "file")

        assertTrue(tracker.observe(snapshot(messages = listOf(message))) is
            ChatGptWebAttachmentSendTracker.Observation.Complete)
    }

    @Test
    fun transportCompletionWaitsForAStableOfficialSnapshotBeforeSending() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 1, snapshot())

        assertTrue(tracker.observeTransport(transport(1, "completed", 1)) is
            ChatGptWebAttachmentSendTracker.Observation.Wait)
        assertTrue(tracker.observe(snapshot(composerReady = false)) is
            ChatGptWebAttachmentSendTracker.Observation.Wait)
        assertTrue(tracker.observe(snapshot()) is ChatGptWebAttachmentSendTracker.Observation.SendPrompt)
        assertEquals(ChatGptWebAttachmentSendTracker.Phase.SENDING, tracker.phase)
    }

    @Test
    fun transportCompletionIsMonotonicForMultipleFiles() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 2, snapshot())

        tracker.observeTransport(transport(2, "completed", 1))
        assertTrue(tracker.observe(snapshot()) is ChatGptWebAttachmentSendTracker.Observation.Wait)
        tracker.observeTransport(transport(1, "completed", 2))
        assertTrue(tracker.observe(snapshot()) is ChatGptWebAttachmentSendTracker.Observation.Wait)
        tracker.observeTransport(transport(3, "completed", 2))

        assertTrue(tracker.observe(snapshot()) is ChatGptWebAttachmentSendTracker.Observation.SendPrompt)
    }

    @Test
    fun transportFailureDoesNotMisreportOfficialCapabilityAsUnavailable() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 1, snapshot())

        tracker.observeTransport(transport(1, "failed", 0))

        assertTrue(tracker.observe(snapshot()) is ChatGptWebAttachmentSendTracker.Observation.Wait)
        assertEquals(ChatGptWebAttachmentSendTracker.Phase.UPLOADING, tracker.phase)
    }

    @Test
    fun armedTransportEvidenceNeverDispatchesBeforeUploadCompletion() {
        val tracker = ChatGptWebAttachmentSendTracker.begin("分析附件", 1, snapshot())

        tracker.observeTransport(transport(1, "armed", 0))

        assertTrue(tracker.observe(snapshot()) is ChatGptWebAttachmentSendTracker.Observation.Wait)
    }

    private fun attachment(id: String, state: String) = ChatGptWebAttachment(
        id = "attachment_$id",
        name = "$id.txt",
        state = state,
        removable = true,
    )

    private fun message(id: String, role: String, text: String, partType: String? = null) = ChatGptWebMessage(
        id = id,
        role = role,
        content = text,
        state = "completed",
        parts = partType?.let { listOf(ChatGptWebMessagePart(it, "fixture")) }.orEmpty(),
    )

    private fun snapshot(
        messages: List<ChatGptWebMessage> = emptyList(),
        attachments: List<ChatGptWebAttachment> = emptyList(),
        composerReady: Boolean = true,
    ) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = composerReady,
        streaming = false,
        currentModel = "极速",
        attachments = attachments,
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.ATTACHMENTS)),
    )

    private fun transport(sequence: Long, state: String, completedCount: Int) =
        ChatGptWebAttachmentTransportEvidence(
            version = 1,
            sequence = sequence,
            state = requireNotNull(ChatGptWebAttachmentTransportState.fromWireValue(state)),
            completedCount = completedCount,
        )
}
