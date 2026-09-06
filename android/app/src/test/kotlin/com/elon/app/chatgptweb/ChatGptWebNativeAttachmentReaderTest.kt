package com.elon.app.chatgptweb

import java.io.ByteArrayInputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebNativeAttachmentReaderTest {
    @Test
    fun readsBoundedSequentialChunksAndClosesAtTheExactSize() {
        val bytes = ByteArray(140_000) { (it % 127).toByte() }
        var closed = false
        val source = object : ByteArrayInputStream(bytes) {
            override fun close() { closed = true; super.close() }
        }
        val reader = ChatGptWebNativeAttachmentReader(bytes.size) { source }
        val first = reader.read(0)
        assertEquals(65_536, first.size)
        val second = reader.read(first.size)
        val third = reader.read(first.size + second.size)
        assertArrayEquals(bytes, first + second + third)
        assertTrue(closed)
        assertThrows(IllegalStateException::class.java) { reader.read(bytes.size) }
    }

    @Test
    fun cancellationAndReplayDoNotReopenTheStagedFile() {
        var opened = 0
        val reader = ChatGptWebNativeAttachmentReader(70_000) {
            opened++
            ByteArrayInputStream(ByteArray(70_000))
        }
        reader.read(0)
        assertThrows(IllegalStateException::class.java) { reader.read(0) }
        reader.close()
        assertThrows(IllegalStateException::class.java) { reader.read(65_536) }
        assertEquals(1, opened)
    }

    @Test
    fun truncatedAndExpandedFilesFailWithoutACompleteByteReceipt() {
        for (actualSize in listOf(2, 4)) {
            val reader = ChatGptWebNativeAttachmentReader(3) { ByteArrayInputStream(ByteArray(actualSize)) }
            assertThrows(IllegalStateException::class.java) { reader.read(0) }
            assertThrows(IllegalStateException::class.java) { reader.read(0) }
        }
    }

    @Test
    fun emptyAndOverLimitFilesAreRejected() {
        for (size in listOf(0, ChatGptWebNativeAttachmentReader.MAX_BYTES + 1)) {
            assertThrows(IllegalArgumentException::class.java) {
                ChatGptWebNativeAttachmentReader(size) { ByteArrayInputStream(byteArrayOf()) }
            }
        }
    }

    @Test
    fun revocationBeforeTheFirstReadNeverOpensTheFile() {
        var opened = 0
        val reader = ChatGptWebNativeAttachmentReader(3) {
            opened++
            ByteArrayInputStream(ByteArray(3))
        }
        reader.revoke()
        assertThrows(IllegalStateException::class.java) { reader.read(0) }
        reader.close()
        reader.close()
        assertEquals(0, opened)
    }

    @Test
    fun revocationDoesNotWaitForBlockedIoOrReturnLateBytes() {
        for (blockAtEndOfFile in listOf(false, true)) {
            val reading = CountDownLatch(1)
            val release = CountDownLatch(1)
            val closed = CountDownLatch(1)
            val workers = Executors.newFixedThreadPool(2)
            val source = object : ByteArrayInputStream(ByteArray(3)) {
                private fun pause() {
                    reading.countDown()
                    check(release.await(5, TimeUnit.SECONDS)) { "test_read_not_released" }
                }
                override fun read(bytes: ByteArray, offset: Int, length: Int): Int {
                    if (!blockAtEndOfFile) pause()
                    return super.read(bytes, offset, length)
                }
                override fun read(): Int {
                    if (blockAtEndOfFile) pause()
                    return super.read()
                }
                override fun close() { closed.countDown(); super.close() }
            }
            val reader = ChatGptWebNativeAttachmentReader(3) { source }
            val outcome = workers.submit<Throwable?> {
                runCatching { reader.read(0) }.exceptionOrNull()
            }
            try {
                assertTrue(reading.await(2, TimeUnit.SECONDS))
                // This completes while the file is still blocked on the other worker.
                workers.submit { reader.revoke() }.get(1, TimeUnit.SECONDS)
                assertEquals(1L, release.count)
                release.countDown()
                assertTrue(outcome.get(2, TimeUnit.SECONDS) is IllegalStateException)
                assertTrue(closed.await(1, TimeUnit.SECONDS))
                assertThrows(IllegalStateException::class.java) { reader.read(0) }
            } finally {
                release.countDown()
                workers.shutdownNow()
                assertTrue(workers.awaitTermination(5, TimeUnit.SECONDS))
                reader.close()
            }
        }
    }
}
