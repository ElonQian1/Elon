package com.elon.app.chatgptweb

import java.util.Base64

internal interface ChatGptWebFileByteDestination {
    fun write(bytes: ByteArray)
    fun publish()
    fun discard()
}

internal class ChatGptWebFileByteTransfer(
    private val destination: ChatGptWebFileByteDestination,
    private val expectedBytes: Long,
) {
    private var sequence = 0L
    private var received = 0L
    private var closed = false

    init { require(expectedBytes in -1..MAX_BYTES) }

    fun append(index: Long, encoded: String): Long {
        check(!closed && index == sequence)
        require(encoded.length in 4..MAX_ENCODED && encoded.length % 4 == 0)
        val bytes = Base64.getDecoder().decode(encoded)
        require(bytes.size in 1..CHUNK_BYTES)
        require(Base64.getEncoder().encodeToString(bytes) == encoded)
        require(received + bytes.size <= MAX_BYTES && (expectedBytes < 0 || received + bytes.size <= expectedBytes))
        destination.write(bytes)
        received += bytes.size
        sequence += 1
        return received
    }

    fun finish(index: Long, total: Long) {
        check(!closed && index == sequence && total == received)
        check(expectedBytes < 0 || received == expectedBytes)
        destination.publish()
        closed = true
    }

    fun cancel() {
        if (closed) return
        closed = true
        destination.discard()
    }

    companion object {
        const val CHUNK_BYTES = 49_152
        const val MAX_ENCODED = CHUNK_BYTES / 3 * 4
        const val MAX_BYTES = 512L * 1024 * 1024
        const val TIMEOUT_MS = 120_000L
        const val COMMAND_TIMEOUT_MS = 135_000L
    }
}
