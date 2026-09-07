package com.elon.app.chatgptweb

import java.io.Closeable
import java.io.File
import java.nio.channels.FileChannel
import java.nio.channels.FileLock
import java.nio.channels.OverlappingFileLockException
import java.nio.file.Files
import java.nio.file.LinkOption.NOFOLLOW_LINKS
import java.nio.file.StandardOpenOption.*

/** Owns only opaque pending-download markers, never credentials or source URLs. */
internal class ChatGptWebDownloadJournal(private val directory: File) {
    data class Recovery(val cleaned: Int, val active: Int, val failed: Int)

    inner class Lease internal constructor(
        val id: String,
        private val channel: FileChannel,
        private val lock: FileLock,
    ) : Closeable {
        @Volatile private var closed = false

        fun complete() = guarded {
            if (!closed) {
                close()
                Files.deleteIfExists(marker(id).toPath())
            }
        }

        // Releasing an unfinished owner deliberately leaves its marker for recovery.
        @Synchronized override fun close() {
            if (closed) return
            closed = true
            try { lock.release() } finally { channel.close() }
        }
    }

    fun begin(id: String): Lease = guarded {
        require(validId(id))
        val channel = FileChannel.open(marker(id).toPath(), CREATE_NEW, READ, WRITE, NOFOLLOW_LINKS)
        try {
            val lock = channel.lock()
            channel.force(true)
            Lease(id, channel, lock)
        } catch (failure: Exception) {
            channel.close()
            throw failure
        }
    }

    fun recover(limit: Int = 128, discardPending: (String) -> Boolean): Recovery {
        require(limit in 1..128)
        val candidates = guarded {
            Files.newDirectoryStream(directory.toPath()).use { entries ->
                entries.asSequence().filter { Files.isRegularFile(it, NOFOLLOW_LINKS) }
                    .map { it.fileName.toString() }.filter { it.endsWith(SUFFIX) }
                    .map { it.removeSuffix(SUFFIX) }.filter(::validId).take(limit).toList()
            }
        }
        var cleaned = 0
        var active = 0
        var failed = 0
        candidates.forEach { id ->
            try {
                val owner = acquireOrphan(id)
                if (owner == null) active++
                else owner.use {
                    // The per-download lock stays held, but slow storage I/O never holds the journal guard.
                    if (discardPending(id)) { owner.complete(); cleaned++ } else failed++
                }
            } catch (_: Exception) { failed++ }
        }
        return Recovery(cleaned, active, failed)
    }

    private fun acquireOrphan(id: String): Lease? = guarded {
        val path = marker(id).toPath()
        if (!Files.isRegularFile(path, NOFOLLOW_LINKS)) return@guarded null
        val channel = FileChannel.open(path, READ, WRITE, NOFOLLOW_LINKS)
        try {
            val lock = try { channel.tryLock() } catch (_: OverlappingFileLockException) { null }
            if (lock == null) { channel.close(); null } else Lease(id, channel, lock)
        } catch (failure: Exception) {
            channel.close()
            throw failure
        }
    }

    private fun marker(id: String) = File(directory, "$id$SUFFIX")

    private fun <T> guarded(action: () -> T): T = synchronized(PROCESS_GUARD) {
        check(directory.isDirectory || directory.mkdirs())
        FileChannel.open(File(directory, "journal.lock").toPath(), CREATE, WRITE, NOFOLLOW_LINKS).use { channel ->
            channel.lock().use { action() }
        }
    }

    companion object {
        private val PROCESS_GUARD = Any()
        private const val SUFFIX = ".pending"
        private val ID = Regex("[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}")
        fun validId(value: String): Boolean = ID.matches(value)
    }
}
