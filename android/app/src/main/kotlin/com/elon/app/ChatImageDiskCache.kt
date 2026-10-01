package com.elon.app

import android.content.Context
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.Closeable
import java.io.File
import java.io.InterruptedIOException
import java.security.MessageDigest
import java.util.concurrent.TimeUnit

/** Original encoded files, independent of the much smaller decoded thumbnail cache. */
internal object ChatImageDiskCache {
    const val MAX_IMAGE_BYTES = 12 * 1024 * 1024
    private const val MAX_CACHE_BYTES = 80L * 1024 * 1024
    private const val TRIM_TARGET_BYTES = 64L * 1024 * 1024
    private val guard = Any()
    private val sourceLocks = Array(32) { Any() }
    private val readers = mutableMapOf<String, Int>()
    private val client = OkHttpClient.Builder()
        .connectTimeout(8, TimeUnit.SECONDS)
        .readTimeout(12, TimeUnit.SECONDS)
        .callTimeout(30, TimeUnit.SECONDS)
        .build()

    class Lease internal constructor(val file: File, private val release: () -> Unit) : Closeable {
        private var closed = false
        @Synchronized override fun close() {
            if (!closed) { closed = true; release() }
        }
    }

    fun readBytes(context: Context, source: String, maxBytes: Int): ByteArray =
        acquire(context, source, maxBytes).use { it.file.readBytes() }

    fun acquire(context: Context, source: String, maxBytes: Int = MAX_IMAGE_BYTES): Lease {
        require(maxBytes in 1..MAX_IMAGE_BYTES)
        if (!source.isRemote()) {
            val file = File(source)
            require(file.isFile && file.length() in 1..maxBytes.toLong()) { "Invalid image file" }
            return Lease(file) {}
        }
        val file = cacheFile(context, source)
        // Downloads coalesce by source; trimming never waits on network IO.
        synchronized(sourceLocks[(source.hashCode() and Int.MAX_VALUE) % sourceLocks.size]) {
            synchronized(guard) {
                if (file.isFile && file.length() > 0) {
                    require(file.length() <= maxBytes) { "Image exceeds caller size limit" }
                    return pin(file)
                }
                require(readers[file.absolutePath] == null) { "Image is in use" }
                file.delete()
            }
            val temp = File.createTempFile("image-", ".part", file.parentFile)
            try {
                download(source, temp, maxBytes)
                synchronized(guard) {
                    check(temp.renameTo(file)) { "Cannot commit image cache" }
                    val lease = pin(file)
                    trim(file.parentFile!!)
                    return lease
                }
            } finally {
                temp.delete()
            }
        }
    }

    fun remove(context: Context, source: String) {
        if (source.isRemote()) synchronized(guard) {
            val file = cacheFile(context, source)
            if (readers[file.absolutePath] == null) file.delete()
        }
    }

    fun sizeBytes(context: Context): Long = synchronized(guard) {
        cachedFiles(cacheDir(context)).sumOf { it.length() }
    }

    /** Open viewers keep their files; clearing does not delete source attachments. */
    fun clearUnused(context: Context): Long = synchronized(guard) {
        var removed = 0L
        cachedFiles(cacheDir(context)).forEach { file ->
            if (readers[file.absolutePath] == null) {
                val bytes = file.length()
                if (file.delete()) removed += bytes
            }
        }
        removed
    }

    private fun pin(file: File): Lease {
        val key = file.absolutePath
        readers[key] = (readers[key] ?: 0) + 1
        file.setLastModified(System.currentTimeMillis())
        return Lease(file) {
            synchronized(guard) {
                val count = (readers[key] ?: 1) - 1
                if (count == 0) readers.remove(key) else readers[key] = count
                trim(file.parentFile!!)
            }
        }
    }

    private fun download(source: String, target: File, maxBytes: Int) {
        client.newCall(Request.Builder().url(source).build()).execute().use { response ->
            check(response.isSuccessful) { "Image HTTP ${response.code}" }
            val body = requireNotNull(response.body)
            require(body.contentLength() <= maxBytes) { "Image exceeds size limit" }
            body.byteStream().use { input ->
                target.outputStream().use { output ->
                    val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                    var total = 0L
                    while (true) {
                        if (Thread.currentThread().isInterrupted) throw InterruptedIOException()
                        val count = input.read(buffer)
                        if (count < 0) break
                        total += count
                        require(total <= maxBytes) { "Image exceeds size limit" }
                        output.write(buffer, 0, count)
                    }
                    require(total > 0) { "Empty image" }
                }
            }
        }
    }

    private fun trim(dir: File) {
        val files = cachedFiles(dir)
        var bytes = files.sumOf { it.length() }
        if (bytes <= MAX_CACHE_BYTES) return
        for (file in files.sortedBy { it.lastModified() }) {
            if (bytes <= TRIM_TARGET_BYTES) break
            if (readers[file.absolutePath] != null) continue
            val length = file.length()
            if (file.delete()) bytes -= length
        }
    }

    private fun cachedFiles(dir: File) = dir.listFiles()?.filter { it.isFile && it.extension == "img" }.orEmpty()
    private fun cacheDir(context: Context) = File(context.applicationContext.cacheDir, "chat_image_cache").apply { mkdirs() }
    private fun cacheFile(context: Context, source: String): File {
        val hash = MessageDigest.getInstance("SHA-256").digest(source.toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it.toInt() and 0xff) }
        return File(cacheDir(context), "$hash.img")
    }
    private fun String.isRemote() = startsWith("http://", true) || startsWith("https://", true)
}
