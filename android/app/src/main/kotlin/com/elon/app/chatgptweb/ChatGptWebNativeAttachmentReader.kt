package com.elon.app.chatgptweb

import java.io.InputStream
import java.util.concurrent.atomic.AtomicBoolean

/** One sequential, revocable read of an already staged, user-selected attachment. */
internal class ChatGptWebNativeAttachmentReader(
    private val size: Int,
    private val open: () -> InputStream,
) : AutoCloseable {
    private var input: InputStream? = null
    private var offset = 0
    private val revoked = AtomicBoolean(false)

    init { require(size in 1..MAX_BYTES) }

    @Synchronized
    fun read(expectedOffset: Int): ByteArray {
        check(!revoked.get() && expectedOffset == offset && offset < size) { "attachment_read_expired" }
        try {
            val source = input ?: open().also { input = it }
            val bytes = ByteArray(minOf(CHUNK_BYTES, size - offset))
            var count = 0
            while (count < bytes.size) {
                check(!revoked.get()) { "attachment_read_expired" }
                val read = source.read(bytes, count, bytes.size - count)
                check(!revoked.get()) { "attachment_read_expired" }
                check(read > 0) { "attachment_size_changed" }
                count += read
            }
            offset += count
            if (offset == size) {
                check(source.read() == -1) { "attachment_size_changed" }
                check(!revoked.get()) { "attachment_read_expired" }
                close()
            }
            return bytes
        } catch (failure: Exception) {
            close()
            throw failure
        }
    }

    // Invalidates an in-flight read without making the UI wait on the I/O monitor.
    fun revoke() { revoked.set(true) }

    @Synchronized
    override fun close() {
        revoke()
        runCatching { input?.close() }
        input = null
    }

    companion object {
        const val CHUNK_BYTES = 64 * 1024
        const val MAX_BYTES = 8 * 1024 * 1024
    }
}
