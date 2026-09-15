package com.elon.app

import com.elon.app.AiConversationShareMediaException.Reason
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.net.URI
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.StandardOpenOption
import java.security.MessageDigest

/** Read-only access to exact selected sources in existing app-owned image/attachment caches. */
internal class AiConversationShareMediaSources(private val cacheDir: File, private val packageName: String) {
    internal class ImageBytes(val bytes: ByteArray, val isPreview: Boolean)

    fun read(source: String, assetHandle: String?, checkActive: () -> Unit = {}): ImageBytes {
        try {
            val file = resolve(source, assetHandle)
            if (!file.isFile) fail(Reason.LOCAL_IMAGE_MISSING_RETRY)
            val length = file.length()
            if (length > AiConversationShareMediaPreparer.MAX_ASSET_BYTES) fail(Reason.ASSET_TOO_LARGE)
            if (length == 0L) fail(Reason.INVALID_IMAGE)
            val bytes = Files.newInputStream(file.toPath(), StandardOpenOption.READ, LinkOption.NOFOLLOW_LINKS).use { input ->
                val output = ByteArrayOutputStream(minOf(length.toInt(), 64 * 1024))
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                while (true) {
                    checkActive()
                    // Read at most one byte past the cap, including files growing during the read.
                    val remaining = AiConversationShareMediaPreparer.MAX_ASSET_BYTES - output.size()
                    val count = input.read(buffer, 0, minOf(buffer.size, remaining + 1))
                    if (count < 0) break
                    if (count == 0) continue
                    if (count > remaining) fail(Reason.ASSET_TOO_LARGE)
                    output.write(buffer, 0, count)
                }
                output.toByteArray()
            }
            if (bytes.isEmpty()) fail(Reason.INVALID_IMAGE)
            return ImageBytes(bytes, file.parentFile?.name == PREVIEW_DIR)
        } catch (_: IOException) {
            // Do not surface paths, content URIs or source URLs through exception messages/causes.
            fail(Reason.LOCAL_IMAGE_MISSING_RETRY)
        } catch (_: SecurityException) {
            fail(Reason.SOURCE_NOT_ALLOWED)
        }
    }

    private fun resolve(source: String, assetHandle: String?): File {
        if (source.isBlank() || source.length > 8_192 || source.any { it.code < 32 }) {
            fail(Reason.SOURCE_NOT_ALLOWED)
        }
        val local = File(source)
        if (local.isAbsolute) return allowFile(local, assetHandle)
        val uri = try { URI(source) } catch (_: Exception) { fail(Reason.SOURCE_NOT_ALLOWED) }
        return when (uri.scheme?.lowercase()) {
            "https", "http" -> {
                if (uri.host.isNullOrBlank() || uri.rawUserInfo != null || uri.rawFragment != null) {
                    fail(Reason.SOURCE_NOT_ALLOWED)
                }
                // ChatImageDiskCache's private mapping, deliberately without readBytes' HTTP fallback.
                allowFile(File(File(cacheDir, DISK_CACHE_DIR), "${sha256(source)}.img"), assetHandle)
            }
            "content" -> allowContentUri(uri, assetHandle)
            "file" -> {
                if (uri.rawAuthority != null || uri.rawQuery != null || uri.rawFragment != null) {
                    fail(Reason.SOURCE_NOT_ALLOWED)
                }
                val file = try { File(uri) } catch (_: Exception) { fail(Reason.SOURCE_NOT_ALLOWED) }
                allowFile(file, assetHandle)
            }
            else -> fail(Reason.SOURCE_NOT_ALLOWED)
        }
    }

    private fun allowContentUri(uri: URI, assetHandle: String?): File {
        if (uri.rawAuthority != "$packageName.fileprovider" || uri.rawQuery != null || uri.rawFragment != null) {
            fail(Reason.SOURCE_NOT_ALLOWED)
        }
        val segments = uri.path.orEmpty().split('/')
        if (segments.size !in 3..5 || segments[0].isNotEmpty() ||
            segments.drop(1).any { it.isBlank() || it == "." || it == ".." || it.contains('\\') ||
                it.any { character -> character.code < 32 } }
        ) fail(Reason.SOURCE_NOT_ALLOWED)
        // Only image-bearing cache roots declared in res/xml/file_paths.xml. Never open an
        // arbitrary ContentResolver provider, which could read private data or perform network I/O.
        val directory = when (segments[1]) {
            "attachment_cache" -> "attachments"
            "chatgpt_web_uploads" -> "chatgpt_web_uploads"
            else -> fail(Reason.SOURCE_NOT_ALLOWED)
        }
        return allowFile(File(File(cacheDir, directory), segments.drop(2).joinToString(File.separator)), assetHandle)
    }

    private fun allowFile(file: File, assetHandle: String?): File {
        if (!file.isAbsolute || file.path.split('/', '\\').any { it == "." || it == ".." }) {
            fail(Reason.SOURCE_NOT_ALLOWED)
        }
        val cache = cacheDir.canonicalFile
        val absolute = file.absoluteFile
        val canonical = file.canonicalFile
        if (!absolute.toPath().startsWith(cacheDir.absoluteFile.toPath())) fail(Reason.SOURCE_NOT_ALLOWED)
        val relative = cacheDir.absoluteFile.toPath().relativize(absolute.toPath()).toString()
        val segments = relative.split(File.separatorChar)
        val directory = segments.firstOrNull() ?: fail(Reason.SOURCE_NOT_ALLOWED)
        if (directory !in CACHE_DIRS || canonical != File(cache, relative)) fail(Reason.SOURCE_NOT_ALLOWED)
        val knownStagingPath = directory == "chatgpt_web_uploads" && segments.size == 4 &&
            STAGING_BATCH.matches(segments[1]) && STAGING_INDEX.matches(segments[2])
        if (segments.size != 2 && !knownStagingPath) fail(Reason.SOURCE_NOT_ALLOWED)
        if (assetHandle != null && directory != PREVIEW_DIR) fail(Reason.SOURCE_NOT_ALLOWED)
        if (directory == PREVIEW_DIR) {
            if (!PREVIEW_NAME.matches(canonical.name) || assetHandle == null ||
                canonical.name != "$assetHandle.jpg"
            ) fail(Reason.SOURCE_NOT_ALLOWED)
        }
        if (directory == DISK_CACHE_DIR && !DISK_NAME.matches(canonical.name)) fail(Reason.SOURCE_NOT_ALLOWED)
        return canonical
    }

    private fun sha256(source: String): String = MessageDigest.getInstance("SHA-256")
        .digest(source.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it.toInt() and 0xff) }

    private fun fail(reason: Reason): Nothing = throw AiConversationShareMediaException(reason)

    private companion object {
        const val PREVIEW_DIR = "chatgpt-web-image-assets-v1"
        const val DISK_CACHE_DIR = "chat_image_cache"
        val CACHE_DIRS = setOf(PREVIEW_DIR, DISK_CACHE_DIR, "pending_attachments", "attachments", "chatgpt_web_uploads")
        val PREVIEW_NAME = Regex("image_[a-f0-9]{16}\\.jpg")
        val DISK_NAME = Regex("[a-f0-9]{64}\\.img")
        val STAGING_BATCH = Regex("[0-9]{1,20}")
        val STAGING_INDEX = Regex("[0-8]")
    }
}
