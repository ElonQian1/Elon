package com.elon.app.chatgptweb

import java.io.ByteArrayInputStream
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
}
