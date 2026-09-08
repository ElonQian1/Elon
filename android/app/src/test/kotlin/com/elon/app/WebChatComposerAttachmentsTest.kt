package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class WebChatComposerAttachmentsTest {
    private val file = WebChatComposerAttachment("attachment_a", "fixture.txt", "ready", true)
    private fun state() = WebChatConsumerState(
        streaming = false, dictationActive = false, composerSections = emptyMap(),
        pageKind = "conversation", pageUrl = "https://chatgpt.com/c/fixture",
        features = emptyList(), commandRequests = emptyList(), attachments = listOf(file),
    )

    @Test fun rendersReadyAndFailedEntriesWithoutStartingAnUpload() {
        val failed = file.copy(id = "attachment_b", state = "failed")
        assertEquals(listOf(file, failed), WebChatComposerAttachments.visible(state().copy(attachments = listOf(file, failed))))
        assertEquals("已添加", WebChatComposerAttachments.status("ready"))
        assertEquals("上传失败", WebChatComposerAttachments.status("failed"))
        assertEquals("处理中", WebChatComposerAttachments.status("unknown"))
    }

    @Test fun hidesDetachedOrStaleProviderData() {
        assertTrue(WebChatComposerAttachments.visible(null).isEmpty())
        assertTrue(WebChatComposerAttachments.visible(state().copy(adapterCurrent = false)).isEmpty())
        assertTrue(WebChatComposerAttachments.visible(state().copy(pageUrl = "")).isEmpty())
    }

    @Test fun removesOnlyCurrentRemovableAttachment() {
        val current = state()
        assertTrue(WebChatComposerAttachments.canRemove(current.pageUrl, file, current))
        assertFalse(WebChatComposerAttachments.canRemove("https://chatgpt.com/c/other", file, current))
        assertFalse(WebChatComposerAttachments.canRemove(current.pageUrl, file, current.copy(attachments = emptyList())))
        assertFalse(WebChatComposerAttachments.canRemove(current.pageUrl, file, current.copy(attachments = listOf(file.copy(removable = false)))))
        assertFalse(WebChatComposerAttachments.canRemove(current.pageUrl, file, current.copy(streaming = true)))
        assertFalse(WebChatComposerAttachments.canRemove(current.pageUrl, file, current.copy(adapterCurrent = false)))
    }

    @Test fun blankAndDuplicateIdsCannotCreateExtraCards() {
        assertEquals(listOf(file), WebChatComposerAttachments.visible(state().copy(attachments = listOf(file, file, file.copy(id = "")))))
    }
}
