package com.elon.app.chatgptweb

import android.content.Context
import android.graphics.BitmapFactory
import java.io.File
import java.util.concurrent.Executors

internal data class ChatGptWebImageAssetEntry(
    val handle: String,
    val localPath: String,
    val width: Int,
    val height: Int,
    val savedAtMs: Long,
)

internal class ChatGptWebImageAssetStore private constructor(
    private val root: File,
    private val executeIo: ((() -> Unit) -> Unit),
) {
    constructor(context: Context) : this(
        root = File(context.cacheDir, DIRECTORY_NAME),
        executeIo = { task -> IO.execute(task) },
    )

    internal constructor(root: File, synchronous: Boolean) : this(
        root = root,
        executeIo = if (synchronous) ({ task -> task() }) else ({ task -> IO.execute(task) }),
    )

    private val listeners = linkedSetOf<() -> Unit>()

    fun resolvePath(handle: String): String? {
        if (!ChatGptWebImageAssetProtocol.validHandle(handle)) return null
        val file = assetFile(handle)
        if (!file.isFile || file.length() !in 1..MAX_ASSET_BYTES) return null
        file.setLastModified(System.currentTimeMillis())
        return file.absolutePath
    }

    fun entries(): List<ChatGptWebImageAssetEntry> = synchronized(this) {
        root.listFiles()
            .orEmpty()
            .asSequence()
            .filter { file -> file.isFile && FILE_NAME.matches(file.name) }
            .sortedByDescending(File::lastModified)
            .take(MAX_FILES)
            .mapNotNull(::entry)
            .toList()
    }

    fun handles(): Set<String> = synchronized(this) {
        root.listFiles()
            .orEmpty()
            .asSequence()
            .filter { file -> file.isFile && FILE_NAME.matches(file.name) }
            .map { file -> file.name.removeSuffix(FILE_SUFFIX) }
            .take(MAX_FILES)
            .toSet()
    }

    fun markGallerySynced(nowMs: Long = System.currentTimeMillis()): Boolean = synchronized(this) {
        runCatching {
            root.mkdirs()
            val marker = File(root, GALLERY_SYNC_MARKER)
            if (!marker.isFile) marker.writeText("v1")
            marker.setLastModified(nowMs)
        }.getOrDefault(false)
    }

    fun hasFreshGallerySync(
        nowMs: Long = System.currentTimeMillis(),
        maxAgeMs: Long = GALLERY_SYNC_FRESH_MS,
    ): Boolean = synchronized(this) {
        if (maxAgeMs <= 0L) return@synchronized false
        val modifiedAt = File(root, GALLERY_SYNC_MARKER).takeIf(File::isFile)?.lastModified() ?: return@synchronized false
        modifiedAt in 1L..nowMs && nowMs - modifiedAt <= maxAgeMs
    }

    fun save(asset: ChatGptWebImageAsset, onComplete: (Boolean) -> Unit) {
        executeIo {
            val saved = synchronized(this) { saveNow(asset) }
            onComplete(saved)
            if (saved) listenersSnapshot().forEach { listener -> listener() }
        }
    }

    fun addListener(listener: () -> Unit) = synchronized(this) {
        listeners += listener
    }

    fun removeListener(listener: () -> Unit) = synchronized(this) {
        listeners -= listener
    }

    private fun saveNow(asset: ChatGptWebImageAsset): Boolean {
        if (!asset.ready || !ChatGptWebImageAssetProtocol.validHandle(asset.handle)) return false
        val bytes = asset.decodedBytes() ?: return false
        if (bytes.size !in MIN_ASSET_BYTES..MAX_ASSET_BYTES.toInt() || !looksLikeJpeg(bytes)) return false
        root.mkdirs()
        val target = assetFile(asset.handle)
        val temporary = File(root, "${target.name}.tmp.${Thread.currentThread().id}")
        return runCatching {
            temporary.outputStream().use { output -> output.write(bytes) }
            if (target.exists() && !target.delete()) error("cannot replace image asset")
            if (!temporary.renameTo(target)) {
                target.outputStream().use { output -> output.write(bytes) }
                temporary.delete()
            }
            target.setLastModified(System.currentTimeMillis())
            trim()
            true
        }.getOrElse {
            temporary.delete()
            false
        }
    }

    private fun entry(file: File): ChatGptWebImageAssetEntry? {
        if (file.length() !in 1..MAX_ASSET_BYTES) return null
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.absolutePath, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        return ChatGptWebImageAssetEntry(
            handle = file.name.removeSuffix(FILE_SUFFIX),
            localPath = file.absolutePath,
            width = bounds.outWidth,
            height = bounds.outHeight,
            savedAtMs = file.lastModified(),
        )
    }

    private fun trim() {
        val files = root.listFiles()
            .orEmpty()
            .filter { file -> file.isFile && FILE_NAME.matches(file.name) }
            .sortedByDescending(File::lastModified)
        var total = files.sumOf(File::length)
        files.forEachIndexed { index, file ->
            if (index < MAX_FILES && total <= MAX_CACHE_BYTES) return@forEachIndexed
            val length = file.length()
            if (file.delete()) total -= length
        }
    }

    private fun listenersSnapshot(): List<() -> Unit> = synchronized(this) { listeners.toList() }

    private fun assetFile(handle: String): File = File(root, handle + FILE_SUFFIX)

    private fun looksLikeJpeg(bytes: ByteArray): Boolean =
        bytes.size >= 4 && bytes[0] == 0xff.toByte() && bytes[1] == 0xd8.toByte() &&
            bytes[bytes.lastIndex - 1] == 0xff.toByte() && bytes.last() == 0xd9.toByte()

    private companion object {
        const val DIRECTORY_NAME = "chatgpt-web-image-assets-v1"
        const val GALLERY_SYNC_MARKER = ".gallery-sync-v1"
        const val GALLERY_SYNC_FRESH_MS = 6L * 60L * 60L * 1_000L
        const val FILE_SUFFIX = ".jpg"
        const val MIN_ASSET_BYTES = 128
        const val MAX_ASSET_BYTES = 1_100_000L
        const val MAX_CACHE_BYTES = 64L * 1024L * 1024L
        const val MAX_FILES = 80
        val FILE_NAME = Regex("^image_[a-f0-9]{16}\\.jpg$")
        val IO = Executors.newSingleThreadExecutor { runnable ->
            Thread(runnable, "chatgpt-image-assets").apply { isDaemon = true }
        }
    }
}
