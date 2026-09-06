package com.elon.app.chatgptweb

import android.content.Context
import android.util.AtomicFile
import java.io.File
import java.io.FileOutputStream

internal interface WebChatConversationSnapshotRepository {
    fun restore(path: String): ChatGptWebSnapshot?
    fun save(path: String, snapshot: ChatGptWebSnapshot)
    fun remove(path: String) = Unit
}

internal class WebChatConversationSnapshotStore(
    context: Context,
    directoryName: String,
    private val fileNameForPath: (String) -> String?,
    private val acceptedFileName: Regex,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val directory = File(context.noBackupFilesDir, safeDirectoryName(directoryName))
    private val memory = object : LinkedHashMap<String, WebChatSnapshotCache>(
        WebChatSnapshotCachePolicy.MAX_MEMORY_ITEMS,
        0.75f,
        true,
    ) {
        override fun removeEldestEntry(
            eldest: MutableMap.MutableEntry<String, WebChatSnapshotCache>?,
        ): Boolean = size > WebChatSnapshotCachePolicy.MAX_MEMORY_ITEMS
    }

    fun restore(path: String): ChatGptWebSnapshot? {
        val target = cacheFile(path) ?: return null
        synchronized(memory) { memory[target.name] }?.let { cached ->
            if (WebChatSnapshotCachePolicy.isUsable(cached.savedAtMs, nowMs())) {
                return cached.snapshot
            }
            synchronized(memory) { memory.remove(target.name) }
        }
        val file = AtomicFile(target)
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return null
        if (bytes.size > WebChatSnapshotCachePolicy.MAX_FILE_BYTES) {
            file.delete()
            return null
        }
        val cache = WebChatSnapshotCacheCodec.decode(bytes.toString(Charsets.UTF_8))
        if (cache == null || !WebChatSnapshotCachePolicy.isUsable(cache.savedAtMs, nowMs())) {
            file.delete()
            return null
        }
        remember(target.name, cache)
        touchIfNeeded(target)
        return cache.snapshot
    }

    fun save(path: String, snapshot: ChatGptWebSnapshot) {
        val target = cacheFile(path) ?: return
        val encoded = WebChatSnapshotCacheCodec.encode(
            WebChatSnapshotCache(snapshot, nowMs()),
        )
        val payload = encoded.toByteArray(Charsets.UTF_8)
        if (
            payload.size > WebChatSnapshotCachePolicy.MAX_FILE_BYTES ||
            (!directory.exists() && !directory.mkdirs())
        ) return
        val cached = WebChatSnapshotCacheCodec.decode(encoded) ?: return
        val file = AtomicFile(target)
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
            remember(target.name, cached)
            trimOldEntries()
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    fun remove(path: String) {
        val target = cacheFile(path) ?: return
        synchronized(memory) { memory.remove(target.name) }
        AtomicFile(target).delete()
    }

    private fun cacheFile(path: String): File? {
        val fileName = fileNameForPath(path)?.takeIf(acceptedFileName::matches) ?: return null
        return File(directory, fileName)
    }

    private fun remember(name: String, cache: WebChatSnapshotCache) {
        synchronized(memory) { memory[name] = cache }
    }

    private fun touchIfNeeded(file: File) {
        val now = nowMs()
        if (WebChatSnapshotCachePolicy.shouldTouch(file.lastModified(), now)) {
            runCatching { file.setLastModified(now) }
        }
    }

    private fun trimOldEntries() {
        val files = directory.listFiles { file ->
            file.isFile && acceptedFileName.matches(file.name)
        }.orEmpty()
        val retained = WebChatSnapshotCachePolicy.retainedNames(files.map { file ->
            WebChatSnapshotCacheEntry(file.name, file.lastModified(), file.length())
        })
        files.filterNot { retained.contains(it.name) }.forEach { file ->
            file.delete()
            synchronized(memory) { memory.remove(file.name) }
        }
    }

    private companion object {
        fun safeDirectoryName(value: String): String {
            require(DIRECTORY_NAME.matches(value)) { "Invalid web chat conversation cache directory" }
            return value
        }

        private val DIRECTORY_NAME = Regex("[a-z0-9-]{4,64}")
    }
}
