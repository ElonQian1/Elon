package com.elon.app

import android.content.Context
import java.io.ByteArrayOutputStream
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

internal object ChatImageDiskCache {
    private const val DIR_NAME = "chat_image_cache"
    private const val MAX_CACHE_BYTES = 80L * 1024L * 1024L
    private const val TRIM_TARGET_BYTES = 64L * 1024L * 1024L
    private const val CONNECT_TIMEOUT_MS = 8_000
    private const val READ_TIMEOUT_MS = 12_000

    fun readBytes(context: Context, source: String, maxBytes: Int): ByteArray {
        if (!source.isRemoteImageSource()) {
            return File(source).readBytes()
        }

        val cacheFile = cacheFile(context.applicationContext, source)
        val cachedLength = cacheFile.takeIf { it.isFile }?.length() ?: 0L
        if (cachedLength in 1..maxBytes.toLong()) {
            cacheFile.setLastModified(System.currentTimeMillis())
            return cacheFile.readBytes()
        }
        if (cachedLength > maxBytes) {
            cacheFile.delete()
        }

        val bytes = downloadBytes(source, maxBytes)
        writeAtomically(cacheFile, bytes)
        trimCache(cacheFile.parentFile)
        return bytes
    }

    fun remove(context: Context, source: String) {
        if (source.isRemoteImageSource()) {
            cacheFile(context.applicationContext, source).delete()
        }
    }

    private fun downloadBytes(source: String, maxBytes: Int): ByteArray {
        val connection = (URL(source).openConnection() as HttpURLConnection).apply {
            connectTimeout = CONNECT_TIMEOUT_MS
            readTimeout = READ_TIMEOUT_MS
            instanceFollowRedirects = true
        }
        try {
            val code = connection.responseCode
            if (code !in 200..299) {
                error("image preview HTTP $code")
            }
            return connection.inputStream.use { input ->
                val output = ByteArrayOutputStream()
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                var total = 0
                while (true) {
                    val read = input.read(buffer)
                    if (read <= 0) break
                    total += read
                    if (total > maxBytes) error("image preview is too large")
                    output.write(buffer, 0, read)
                }
                output.toByteArray()
            }
        } finally {
            connection.disconnect()
        }
    }

    private fun writeAtomically(cacheFile: File, bytes: ByteArray) {
        val dir = cacheFile.parentFile ?: return
        dir.mkdirs()
        val temp = File(dir, "${cacheFile.name}.tmp.${Thread.currentThread().id}")
        runCatching {
            temp.outputStream().use { it.write(bytes) }
            if (cacheFile.exists()) cacheFile.delete()
            if (!temp.renameTo(cacheFile)) {
                cacheFile.outputStream().use { it.write(bytes) }
                temp.delete()
            }
        }.onFailure {
            temp.delete()
        }
    }

    private fun trimCache(dir: File?) {
        val files = dir?.listFiles()?.filter { it.isFile } ?: return
        var total = 0L
        for (file in files) {
            total += file.length()
        }
        if (total <= MAX_CACHE_BYTES) return

        for (file in files.sortedBy { it.lastModified() }) {
            if (total <= TRIM_TARGET_BYTES) return
            val length = file.length()
            if (file.delete()) {
                total -= length
            }
        }
    }

    private fun cacheFile(context: Context, source: String): File {
        val dir = File(context.cacheDir, DIR_NAME).apply { mkdirs() }
        return File(dir, "${sha256(source)}.img")
    }

    private fun sha256(value: String): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(value.toByteArray(Charsets.UTF_8))
        return digest.joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }
    }

    private fun String.isRemoteImageSource(): Boolean {
        return startsWith("http://", ignoreCase = true) || startsWith("https://", ignoreCase = true)
    }
}
