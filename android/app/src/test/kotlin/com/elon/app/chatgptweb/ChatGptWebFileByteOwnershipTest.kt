package com.elon.app.chatgptweb

import java.io.IOException
import java.util.Base64
import java.util.UUID
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class ChatGptWebFileByteOwnershipTest {
    @get:Rule val temporary = TemporaryFolder()
    private val journal get() = ChatGptWebDownloadJournal(temporary.root)
    private class Destination : ChatGptWebFileByteDestination {
        var published = false
        var discarded = false
        var failPublish = false
        var failDiscard = false
        override fun write(bytes: ByteArray) = Unit
        override fun publish() { check(!failPublish); published = true }
        override fun discard() { check(!failDiscard); discarded = true }
    }

    @Test fun destinationAllocationIsProtectedAndPartialAllocationCanRecover() {
        assertTrue(runCatching {
            ChatGptWebFileByteOwnership.open(journal, UUID.randomUUID().toString()) {
                assertEquals(1, journal.recover { error("allocation is active") }.active)
                throw IOException("output could not be opened")
            }
        }.isFailure)
        assertEquals(1, journal.recover { true }.cleaned)
    }

    @Test fun exactTransferPublicationRemovesMarkerAndLateCancelCannotDiscard() {
        val destination = Destination()
        val owned = ChatGptWebFileByteOwnership.open(journal, UUID.randomUUID().toString()) { destination }
        val transfer = ChatGptWebFileByteTransfer(owned, 1)
        transfer.append(0, Base64.getEncoder().encodeToString(byteArrayOf(1)))
        assertEquals(1, journal.recover { error("still writing") }.active)
        transfer.finish(1, 1)
        transfer.cancel()
        assertTrue(destination.published)
        assertFalse(destination.discarded)
        assertEquals(0, journal.recover { error("already complete") }.cleaned)
    }

    @Test fun failedPublicationIsDiscardedBeforeOwnershipCompletes() {
        val destination = Destination().apply { failPublish = true }
        val owned = ChatGptWebFileByteOwnership.open(journal, UUID.randomUUID().toString()) { destination }
        val transfer = ChatGptWebFileByteTransfer(owned, 0)
        assertTrue(runCatching { transfer.finish(0, 0) }.isFailure)
        assertEquals(1, journal.recover { error("waiting for cancellation") }.active)
        transfer.cancel()
        assertTrue(destination.discarded)
        assertEquals(0, journal.recover { error("discarded") }.cleaned)
    }

    @Test fun failedCancellationLeavesAnUnlockedRecoverableMarker() {
        val destination = Destination().apply { failDiscard = true }
        val owned = ChatGptWebFileByteOwnership.open(journal, UUID.randomUUID().toString()) { destination }
        assertTrue(runCatching { owned.discard() }.isFailure)
        assertEquals(1, journal.recover { destination.failDiscard = false; destination.discard(); true }.cleaned)
        assertTrue(destination.discarded)
    }
}
