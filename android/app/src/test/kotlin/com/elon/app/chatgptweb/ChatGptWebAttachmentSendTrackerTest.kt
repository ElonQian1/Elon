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
    ) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "极速",
        attachments = attachments,
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.ATTACHMENTS)),
    )
}
