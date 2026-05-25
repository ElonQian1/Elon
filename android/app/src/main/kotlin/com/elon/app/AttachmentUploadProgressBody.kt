package com.elon.app

import okhttp3.MediaType
import okhttp3.RequestBody
import okio.BufferedSink
import java.io.File

internal data class AttachmentUploadProgress(
    val attachmentIndex: Int,
    val attachmentCount: Int,
    val bytesWritten: Long,
    val totalBytes: Long
) {
    val percent: Int
        get() {
            if (totalBytes <= 0L) return 0
            return ((bytesWritten * 100) / totalBytes).toInt().coerceIn(0, 100)
        }
}

internal class AttachmentUploadProgressBody(
    private val file: File,
    private val mediaType: MediaType?,
    private val attachmentIndex: Int,
    private val attachmentCount: Int,
    private val onProgress: (AttachmentUploadProgress) -> Unit
) : RequestBody() {
    override fun contentType(): MediaType? = mediaType

    override fun contentLength(): Long = file.length()

    override fun writeTo(sink: BufferedSink) {
        val total = contentLength()
        var written = 0L
        onProgress(progress(written, total))
        file.inputStream().use { input ->
            val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
            while (true) {
                val read = input.read(buffer)
                if (read <= 0) break
                sink.write(buffer, 0, read)
                written += read
                onProgress(progress(written, total))
            }
        }
    }

    private fun progress(written: Long, total: Long): AttachmentUploadProgress {
        return AttachmentUploadProgress(
            attachmentIndex = attachmentIndex,
            attachmentCount = attachmentCount,
            bytesWritten = written,
            totalBytes = total
        )
    }
}
