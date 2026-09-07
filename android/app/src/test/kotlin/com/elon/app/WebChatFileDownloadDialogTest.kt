package com.elon.app

import com.elon.app.WebChatFileDownloadState.Stage
import org.junit.Assert.*
import org.junit.Test
import java.io.File

class WebChatFileDownloadDialogTest {
    @Test fun statusDistinguishesSavingQueuedCancelledAndUnknownResults() {
        assertEquals("正在保存", WebChatFileDownloadPresentation.status(WebChatFileDownloadState("r", Stage.SAVING)))
        assertEquals("已交给系统下载", WebChatFileDownloadPresentation.status(WebChatFileDownloadState("r", Stage.QUEUED)))
        assertEquals("已保存到下载目录", WebChatFileDownloadPresentation.status(WebChatFileDownloadState("r", Stage.SAVED)))
        assertEquals("已取消下载", WebChatFileDownloadPresentation.status(WebChatFileDownloadState("r", Stage.CANCELLED)))
        assertTrue(WebChatFileDownloadPresentation.status(WebChatFileDownloadState("r", Stage.UNCONFIRMED)).contains("尚未确认"))
        assertTrue(WebChatFileDownloadPresentation.status(WebChatFileDownloadState("r", Stage.FAILED), "download_storage_failed").contains("存储空间"))
    }

    @Test fun unknownLengthHasNoFakePercentageAndKnownBytesAreVisible() {
        val unknown = WebChatFileDownloadState("r", Stage.TRANSFERRING, 1024)
        assertNull(unknown.progressPercent)
        assertEquals("1.0 KB", WebChatFileDownloadPresentation.bytes(unknown))
        val known = unknown.copy(totalBytes = 2048)
        assertEquals(50, known.progressPercent)
        assertEquals("1.0 KB / 2.0 KB", WebChatFileDownloadPresentation.bytes(known))
    }

    @Test fun collapseAndCoordinatorDisposalDoNotSendDownloadCancellation() {
        val root = sequenceOf(File("src/main/kotlin/com/elon/app"), File("android/app/src/main/kotlin/com/elon/app"),
            File("app/src/main/kotlin/com/elon/app")).first { it.isDirectory }
        val source = File(root, "WebChatFileDownloadDialog.kt").readText()
        assertTrue(source.contains("owner.cancelFileDownload(requestId)"))
        assertTrue(source.contains("web-chat-file-download-cancel"))
        assertTrue(source.contains("web-chat-file-download-collapse"))
        assertFalse(source.substringAfter("fun dismiss() {").contains("cancelFileDownload"))
        val coordinator = File(root, "WebChatConversationFilesCoordinator.kt").readText()
        assertTrue(coordinator.contains("web-chat-conversation-files-download-progress"))
        assertTrue(coordinator.contains("downloads.show(owner, download.requestId)"))
        assertTrue(coordinator.contains("downloads.show(owner, request.requestId)"))
        assertFalse(coordinator.substringAfter("fun cancel() {").contains("cancelFileDownload"))
    }
}
