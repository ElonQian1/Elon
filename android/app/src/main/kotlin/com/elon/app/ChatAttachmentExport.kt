package com.elon.app

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.net.Uri
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.ByteArrayOutputStream
import java.io.File
import java.security.MessageDigest

internal data class ChatAttachmentExport(val uri: Uri, val mimeType: String) {
    companion object {
        const val MAX_BYTES = 12 * 1024 * 1024
        private const val MAX_FILES = 6
        private const val CACHE_BYTES = 192L * 1024 * 1024
        private const val RETENTION_MS = 24L * 60 * 60 * 1000

        fun prepare(context: Context, attachments: List<ChatAttachment>): List<ChatAttachmentExport> {
            require(attachments.size in 1..MAX_FILES) { "一次最多复制或转发 6 个附件" }
            return attachments.map { attachment ->
                val source = attachment.playbackSource() ?: error("附件暂不可用，请重新加载")
                val bytes = if (source.startsWith("https://", true) || source.startsWith("http://", true)) {
                    ChatImageDiskCache.readBytes(context, source, MAX_BYTES)
                } else {
                    val file = File(source).canonicalFile
                    val mediaRoots = listOf("pending_attachments", "attachments", "voice_attachments", "external_shares",
                        "chatgpt_web_uploads", "chat_record_media", "message_exports")
                    require(mediaRoots.any { file.path.startsWith(File(context.cacheDir, it).canonicalPath + File.separator) }) {
                        "附件地址不可分享，请重新选择文件"
                    }
                    require(file.isFile && file.length() in 1..MAX_BYTES.toLong()) { "附件为空或超过 12 MB" }
                    file.inputStream().use { input ->
                        val output = ByteArrayOutputStream()
                        val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                        var count = input.read(buffer)
                        while (count != -1) {
                            require(output.size() + count <= MAX_BYTES) { "附件超过 12 MB" }
                            output.write(buffer, 0, count)
                            count = input.read(buffer)
                        }
                        output.toByteArray()
                    }
                }
                require(bytes.isNotEmpty() && bytes.size <= MAX_BYTES) { "附件为空或超过 12 MB" }
                val mime = if (attachment.isImage()) {
                    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
                    require(bounds.outWidth > 0 && bounds.outHeight > 0 && bounds.outMimeType?.startsWith("image/") == true) {
                        "图片文件无效，请重新加载"
                    }
                    bounds.outMimeType
                } else attachment.mimeType?.takeIf { it.matches(Regex("[a-zA-Z0-9.+-]+/[a-zA-Z0-9.+-]+")) }
                    ?: "application/octet-stream"
                store(context, bytes, mime, attachment.fileName ?: attachment.displayName)
            }
        }

        @Synchronized private fun store(context: Context, bytes: ByteArray, mime: String, name: String?): ChatAttachmentExport {
            val directory = File(context.cacheDir, "message_exports").apply { mkdirs() }
            val now = System.currentTimeMillis()
            // Keep recently granted files alive; only expire our own old export cache.
            directory.listFiles()?.filter { it.isFile && now - it.lastModified() > RETENTION_MS }?.forEach { it.delete() }
            val digest = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it.toInt() and 255) }
            val extension = MimeTypeMap.getSingleton().getExtensionFromMimeType(mime)
                ?: name?.substringAfterLast('.', "")?.takeIf { it.matches(Regex("[a-zA-Z0-9]{1,10}")) } ?: "bin"
            val label = name?.substringBeforeLast('.')?.replace(Regex("[^\\p{L}\\p{N}_-]"), "_")?.take(60)?.takeIf { it.isNotBlank() } ?: "attachment"
            val file = File(directory, "${digest}_${label}.$extension")
            if (!file.exists()) {
                require(directory.listFiles().orEmpty().sumOf { it.length() } + bytes.size <= CACHE_BYTES) {
                    "附件分享缓存已满，请稍后重试"
                }
                val temp = File.createTempFile("export-", ".tmp", directory)
                try {
                    temp.writeBytes(bytes)
                    check(temp.renameTo(file)) { "附件准备失败，请重试" }
                } finally { temp.delete() }
            }
            file.setLastModified(now)
            return ChatAttachmentExport(FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file), mime)
        }

        fun clip(context: Context, files: List<ChatAttachmentExport>, text: String): ClipData {
            require(files.isNotEmpty())
            return ClipData.newUri(context.contentResolver, "聊天附件", files.first().uri).apply {
                files.drop(1).forEach { addItem(ClipData.Item(it.uri)) }
                if (text.isNotBlank()) addItem(ClipData.Item(text))
            }
        }

        fun intent(context: Context, files: List<ChatAttachmentExport>, text: String): Intent {
            require(files.isNotEmpty())
            val types = files.map { it.mimeType }.distinct()
            return Intent(if (files.size == 1) Intent.ACTION_SEND else Intent.ACTION_SEND_MULTIPLE).apply {
                type = if (types.size == 1) types.single() else if (types.all { it.startsWith("image/") }) "image/*" else "*/*"
                if (files.size == 1) putExtra(Intent.EXTRA_STREAM, files.single().uri)
                else putParcelableArrayListExtra(Intent.EXTRA_STREAM, ArrayList(files.map { it.uri }))
                if (text.isNotBlank()) putExtra(Intent.EXTRA_TEXT, text)
                clipData = clip(context, files, text)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
        }
    }
}
