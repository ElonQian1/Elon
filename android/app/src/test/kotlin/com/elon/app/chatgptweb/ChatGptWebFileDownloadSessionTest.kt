package com.elon.app.chatgptweb

import com.elon.app.WebChatFileDownloadState.Stage
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebFileDownloadSessionTest {
    @Test fun progressBelongsToOneRequestAndOnlyThatRequestCanCancel() {
        val session = ChatGptWebFileDownloadSession()
        assertTrue(session.begin("lease-a", "request-a"))
        assertFalse(session.begin("lease-b", "request-b"))
        assertNull(session.requestCancel("request-b"))
        session.update("lease-b", Stage.TRANSFERRING, 90, 100)
        assertEquals(Stage.PREPARING, session.snapshot()?.stage)
        session.update("lease-a", Stage.TRANSFERRING, 30, 100)
        assertEquals(30, session.snapshot()?.progressPercent)
        assertEquals("lease-a", session.requestCancel("request-a"))
        assertEquals(Stage.CANCELLING, session.snapshot()?.stage)
        assertFalse(session.snapshot()!!.canCancel)
        session.update("lease-a", Stage.CANCELLED)
        assertEquals(Stage.CANCELLED, session.snapshot()?.stage)
        assertNull(session.requestCancel("request-a"))
    }

    @Test fun publishedFileWinsTheCancellationRaceAndLaterCleanupCannotRewriteIt() {
        val session = ChatGptWebFileDownloadSession()
        session.begin("a", "r")
        session.update("a", Stage.TRANSFERRING, 100, 100)
        session.update("a", Stage.SAVING, 100, 100)
        assertFalse(session.snapshot()!!.canCancel)
        session.requestCancel("r")
        session.pageCancelled("a")
        session.pageCancelled("a")
        assertEquals(Stage.CANCELLING, session.snapshot()?.stage)
        assertFalse(session.begin("b", "rb"))
        session.update("a", Stage.SAVED, 100, 100)
        session.update("a", Stage.CANCELLED)
        session.update("a", Stage.FAILED)
        assertEquals(Stage.SAVED, session.snapshot()?.stage)
        assertEquals(100L, session.snapshot()?.receivedBytes)
    }

    @Test fun pageAbortBeforeNativeTransferStartsTerminatesPreparationOnlyForItsLease() {
        val session = ChatGptWebFileDownloadSession()
        session.begin("a", "r")
        session.pageCancelled("old")
        assertEquals(Stage.PREPARING, session.snapshot()?.stage)
        session.pageCancelled("a")
        assertEquals(Stage.FAILED, session.snapshot()?.stage)
    }

    @Test fun oldEventsCannotOverwriteTheNextDownloadOrReviveTerminalFailure() {
        val session = ChatGptWebFileDownloadSession()
        session.begin("a", "ra")
        session.update("a", Stage.FAILED)
        session.update("a", Stage.TRANSFERRING, 2)
        assertEquals(Stage.FAILED, session.snapshot()?.stage)
        assertTrue(session.begin("b", "rb"))
        session.update("a", Stage.SAVED, 99)
        assertEquals("rb", session.snapshot()?.requestId)
        assertEquals(Stage.PREPARING, session.snapshot()?.stage)
    }

    @Test fun automaticCleanupIsFailureNotUserCancellationAndInvalidSizesAreIgnored() {
        val session = ChatGptWebFileDownloadSession()
        session.begin("a", "r")
        session.update("a", Stage.TRANSFERRING, 5, 20)
        session.update("a", Stage.TRANSFERRING, -1, 20)
        session.update("a", Stage.TRANSFERRING, ChatGptWebFileByteTransfer.MAX_BYTES + 1, 20)
        session.update("a", Stage.TRANSFERRING, 3, -1)
        assertEquals(5L, session.snapshot()?.receivedBytes)
        assertEquals(20L, session.snapshot()?.totalBytes)
        session.update("a", Stage.CANCELLED)
        assertEquals(Stage.FAILED, session.snapshot()?.stage)
    }

    @Test fun missingPageReplyCannotKeepTheDownloadBusyForever() {
        var now = 0L
        val session = ChatGptWebFileDownloadSession { now }
        session.begin("a", "ra")
        now = 25_000
        assertEquals(Stage.FAILED, session.snapshot()?.stage)
        assertTrue(session.begin("b", "rb"))
        session.update("b", Stage.SAVING, 10, 10)
        now += ChatGptWebFileByteTransfer.COMMAND_TIMEOUT_MS
        assertEquals(Stage.UNCONFIRMED, session.snapshot()?.stage)
        session.update("b", Stage.SAVED, 10, 10)
        assertEquals(Stage.SAVED, session.snapshot()?.stage)
    }
}
