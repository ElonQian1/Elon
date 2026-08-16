package com.elon.app.chatgptweb

import android.content.Context
import android.util.AtomicFile
import java.io.File
import java.io.FileOutputStream

internal interface WebChatConversationSnapshotRepository {
    fun restore(path: String): ChatGptWebSnapshot?
    fun save(path: String, snapshot: ChatGptWebSnapshot)
}

internal class WebChatConversationSnapshotStore(
    context: Context,
    directoryName: String,
    private val fileNameForPath: (String) -> String?,
    private val acceptedFileName: Regex,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val directory = File(context.noBackupFilesDir, safeDirectoryName(directoryName))

    fun restore(path: String): ChatGptWebSnapshot? {
        val file = atomicFile(path) ?: return null
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return null
        if (bytes.size > MAX_BYTES) return null
        val cache = WebChatSnapshotCacheCodec.decode(bytes.toString(Charsets.UTF_8)) ?: return null
        if (nowMs() - cache.savedAtMs !in 0..MAX_AGE_MS) return null
        return cache.snapshot
    }

    fun save(path: String, snapshot: ChatGptWebSnapshot) {
        val file = atomicFile(path) ?: return
        val payload = WebChatSnapshotCacheCodec.encode(
            WebChatSnapshotCache(snapshot, nowMs()),
        ).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_BYTES || (!directory.exists() && !directory.mkdirs())) return
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
            trimOldEntries()
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    private fun atomicFile(path: String): AtomicFile? {
        val fileName = fileNameForPath(path)?.takeIf(acceptedFileName::matches) ?: return null
        return AtomicFile(File(directory, fileName))
    }

    private fun trimOldEntries() {
        directory.listFiles { file -> file.isFile && acceptedFileName.matches(file.name) }
            .orEmpty()
            .sortedByDescending(File::lastModified)
            .drop(MAX_ITEMS)
            .forEach(File::delete)
    }

    private companion object {
        fun safeDirectoryName(value: String): String {
            require(DIRECTORY_NAME.matches(value)) { "Invalid web chat conversation cache directory" }
            return value
        }

        private val DIRECTORY_NAME = Regex("[a-z0-9-]{4,64}")
        private const val MAX_ITEMS = 48
        private const val MAX_BYTES = 512 * 1024
        private const val MAX_AGE_MS = 7L * 24L * 60L * 60L * 1_000L
    }
}
