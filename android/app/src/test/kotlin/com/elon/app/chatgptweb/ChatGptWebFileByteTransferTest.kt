package com.elon.app.chatgptweb

import java.io.ByteArrayOutputStream
import java.util.Base64
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebFileByteTransferTest {
    private class Destination : ChatGptWebFileByteDestination {
        val bytes = ByteArrayOutputStream()
        var published = false
        var discarded = false
        var failPublish = false
        override fun write(bytes: ByteArray) { this.bytes.write(bytes) }
        override fun publish() { check(!failPublish); published = true }
        override fun discard() { discarded = true; bytes.reset() }
    }
    private fun encoded(value: ByteArray) = Base64.getEncoder().encodeToString(value)
    private fun rejected(block: () -> Unit) { assertTrue(runCatching(block).isFailure) }

    @Test fun bytesAreSavedInOrderAndOnlyPublishedAfterExactCompletion() {
        val destination = Destination()
        val transfer = ChatGptWebFileByteTransfer(destination, 5)
        assertEquals(2L, transfer.append(0, encoded(byteArrayOf(0, 1))))
        assertFalse(destination.published)
        assertEquals(5L, transfer.append(1, encoded(byteArrayOf(2, -1, 4))))
        transfer.finish(2, 5)
        assertArrayEquals(byteArrayOf(0, 1, 2, -1, 4), destination.bytes.toByteArray())
        assertTrue(destination.published)
        transfer.cancel()
        assertFalse(destination.discarded)
        rejected { transfer.finish(2, 5) }
        rejected { transfer.append(2, encoded(byteArrayOf(5))) }
    }

    @Test fun duplicateOutOfOrderAndTruncatedTransfersCannotPublish() {
        val destination = Destination()
        val transfer = ChatGptWebFileByteTransfer(destination, 4)
        rejected { transfer.append(1, encoded(byteArrayOf(1))) }
        transfer.append(0, encoded(byteArrayOf(1, 2)))
        rejected { transfer.append(0, encoded(byteArrayOf(1, 2))) }
        rejected { transfer.finish(1, 2) }
        rejected { transfer.finish(1, 4) }
        rejected { transfer.append(1, encoded(byteArrayOf(3, 4, 5))) }
        assertFalse(destination.published)
        transfer.cancel()
        assertTrue(destination.discarded)
        assertEquals(0, destination.bytes.size())
    }

    @Test fun streamingWithoutContentLengthStillVerifiesSequenceAndByteCount() {
        val destination = Destination()
        val transfer = ChatGptWebFileByteTransfer(destination, -1)
        val block = ByteArray(ChatGptWebFileByteTransfer.CHUNK_BYTES) { (it % 256).toByte() }
        transfer.append(0, encoded(block))
        rejected { transfer.finish(1, block.size.toLong() - 1) }
        transfer.finish(1, block.size.toLong())
        assertArrayEquals(block, destination.bytes.toByteArray())
    }

    @Test fun malformedOrOversizedPacketsDoNotWriteBytes() {
        val destination = Destination()
        val transfer = ChatGptWebFileByteTransfer(destination, -1)
        for (data in listOf("", "!!!!", "AQ", "AQ=\n", "AR==",
            encoded(ByteArray(ChatGptWebFileByteTransfer.CHUNK_BYTES + 1)))) {
            rejected { transfer.append(0, data) }
        }
        assertEquals(0, destination.bytes.size())
        rejected { ChatGptWebFileByteTransfer(destination, -2) }
        rejected { ChatGptWebFileByteTransfer(destination, ChatGptWebFileByteTransfer.MAX_BYTES + 1) }
    }

    @Test fun emptyFilesAndFailedPublicationHaveDistinctOutcomes() {
        val empty = Destination()
        ChatGptWebFileByteTransfer(empty, 0).finish(0, 0)
        assertTrue(empty.published)
        val failed = Destination().apply { failPublish = true }
        val transfer = ChatGptWebFileByteTransfer(failed, 1)
        transfer.append(0, encoded(byteArrayOf(1)))
        rejected { transfer.finish(1, 1) }
        transfer.cancel()
        assertFalse(failed.published)
        assertTrue(failed.discarded)
    }

}
