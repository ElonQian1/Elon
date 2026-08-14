package com.elon.app.googleweb

import android.content.Context
import android.util.AtomicFile
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.WebChatSnapshotCache
import com.elon.app.chatgptweb.WebChatSnapshotCacheCodec
import java.io.File
import java.io.FileOutputStream

internal class GoogleWebConversationSnapshotStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val directory = File(context.noBackupFilesDir, DIRECTORY_NAME)

    fun restore(path: String): ChatGptWebSnapshot? {
        val fileName = fileName(path) ?: return null
        val bytes = runCatching { AtomicFile(File(directory, fileName)).readFully() }
            .getOrNull()
            ?: return null
        if (bytes.size > MAX_BYTES) return null
        val cache = WebChatSnapshotCacheCodec.decode(bytes.toString(Charsets.UTF_8)) ?: return null
        if (nowMs() - cache.savedAtMs !in 0..MAX_AGE_MS) return null
        return cache.snapshot
    }

    fun save(path: String, snapshot: ChatGptWebSnapshot) {
        val fileName = fileName(path) ?: return
        val payload = WebChatSnapshotCacheCodec.encode(
            WebChatSnapshotCache(snapshot, nowMs()),
        ).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_BYTES || (!directory.exists() && !directory.mkdirs())) return
        val file = AtomicFile(File(directory, fileName))
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
            trimOldEntries()
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    private fun trimOldEntries() {
        directory.listFiles { file -> file.isFile && FILE_NAME.matches(file.name) }
            .orEmpty()
            .sortedByDescending(File::lastModified)
            .drop(MAX_ITEMS)
            .forEach(File::delete)
    }

    internal companion object {
        fun fileName(path: String): String? = PATH.matchEntire(path)
            ?.groupValues
            ?.get(1)
            ?.let { "google-web-conversation-$it-v1.json" }

        private const val DIRECTORY_NAME = "google-web-conversation-snapshots-v1"
        private const val MAX_ITEMS = 48
        private const val MAX_BYTES = 512 * 1024
        private const val MAX_AGE_MS = 7L * 24L * 60L * 60L * 1_000L
        private val PATH = Regex("^/google-ai-mode/conversation/([a-f0-9]{64})$")
        private val FILE_NAME = Regex("^google-web-conversation-[a-f0-9]{64}-v1\\.json$")
    }
}
