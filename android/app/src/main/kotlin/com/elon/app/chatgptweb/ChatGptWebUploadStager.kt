package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
import androidx.core.content.FileProvider
import com.elon.app.PendingAttachment
import java.io.File
import java.util.Locale

internal class ChatGptWebUploadStager(private val context: Context) {
    fun stage(attachments: List<PendingAttachment>): List<Uri> {
        require(attachments.isNotEmpty())
        require(attachments.size <= MAX_UPLOADS)
        cleanupExpiredFiles()
        val directory = File(context.cacheDir, DIRECTORY).apply { mkdirs() }
        val batch = File(directory, System.currentTimeMillis().toString()).apply { mkdirs() }
        return runCatching {
            attachments.mapIndexed { index, attachment ->
                require(attachment.file.isFile && attachment.file.length() in 1..MAX_UPLOAD_BYTES)
                val name = ChatGptWebUploadPolicy.stagedName(
                    displayName = attachment.displayName,
                    fallbackName = attachment.fileName,
                    index = index,
                )
                val target = File(File(batch, index.toString()).apply { mkdirs() }, name)
                attachment.file.inputStream().use { input ->
                    target.outputStream().use(input::copyTo)
                }
                require(target.length() == attachment.file.length()) { "Staged attachment length mismatch" }
                FileProvider.getUriForFile(
                    context,
                    "${context.packageName}.fileprovider",
                    target,
                )
            }
        }.onFailure { runCatching(batch::deleteRecursively) }.getOrThrow()
    }

    private fun cleanupExpiredFiles() {
        val cutoff = System.currentTimeMillis() - MAX_STAGED_AGE_MS
        File(context.cacheDir, DIRECTORY).listFiles()?.forEach { file ->
            if (file.lastModified() < cutoff) runCatching(file::deleteRecursively)
        }
    }

    private companion object {
        const val DIRECTORY = "chatgpt_web_uploads"
        const val MAX_UPLOADS = 9
        const val MAX_UPLOAD_BYTES = 8L * 1024L * 1024L
        const val MAX_STAGED_AGE_MS = 24L * 60L * 60L * 1_000L
    }
}

internal object ChatGptWebUploadPolicy {
    fun stagedName(displayName: String, fallbackName: String, index: Int): String {
        val display = displayName.trim()
        val fallback = fallbackName.trim()
        val source = when {
            display.isBlank() -> fallback
            display.substringAfterLast('.', "").isBlank() && fallback.substringAfterLast('.', "").isNotBlank() -> {
                "$display.${fallback.substringAfterLast('.')}"
            }
            else -> display
        }.ifBlank { "attachment_${index + 1}" }
        val normalized = source
            .replace(UNSAFE_FILENAME, "_")
            .trim('.', ' ', '_')
            .take(MAX_FILENAME_LENGTH)
        return normalized.ifBlank { "attachment_${index + 1}" }.lowercase(Locale.ROOT)
    }

    private const val MAX_FILENAME_LENGTH = 120
    private val UNSAFE_FILENAME = Regex("[^A-Za-z0-9._()\\-\\u4e00-\\u9fff]")
}
