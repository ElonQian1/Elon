package com.elon.app.chatgptweb

import java.io.InputStream

/** One sequential, revocable read of an already staged, user-selected attachment. */
internal class ChatGptWebNativeAttachmentReader(
    private val size: Int,
    private val open: () -> InputStream,
) : AutoCloseable {
    private var input: InputStream? = null
    private var offset = 0
    private var closed = false

    init { require(size in 1..MAX_BYTES) }

    @Synchronized
    fun read(expectedOffset: Int): ByteArray {
        check(!closed && expectedOffset == offset && offset < size) { "attachment_read_expired" }
        try {
            val source = input ?: open().also { input = it }
            val bytes = ByteArray(minOf(CHUNK_BYTES, size - offset))
            var count = 0
            while (count < bytes.size) {
                val read = source.read(bytes, count, bytes.size - count)
                check(read > 0) { "attachment_size_changed" }
                count += read
            }
            offset += count
            if (offset == size) {
                check(source.read() == -1) { "attachment_size_changed" }
                close()
            }
            return bytes
        } catch (failure: Exception) {
            close()
            throw failure
        }
    }

    @Synchronized
    override fun close() {
        closed = true
        runCatching { input?.close() }
        input = null
    }

    companion object {
        const val CHUNK_BYTES = 64 * 1024
        const val MAX_BYTES = 8 * 1024 * 1024
    }
}
