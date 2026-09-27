package com.elon.app.sharing

import android.content.Context
import android.content.Intent
import android.net.Uri
import com.elon.app.PendingAttachment
import com.elon.app.chatrecords.WechatArchive
import com.elon.app.displayNameForUri
import java.io.File

internal object ChatRecordShareImport {
    fun import(context: Context, intent: Intent, uris: List<Uri>, id: String, dir: File): ShareDraft? {
        val archives = uris.filter { uri ->
            val mime = context.contentResolver.getType(uri).orEmpty().ifBlank { intent.type.orEmpty() }
            mime in setOf("application/zip", "application/x-zip-compressed") ||
                displayNameForUri(context, uri).orEmpty().endsWith(".zip", true)
        }
        if (archives.isEmpty()) return null
        require(archives.size == 1 && uris.size == 1) { "请每次分享一份微信聊天记录 ZIP" }
        val uri = archives.single(); require(uri.scheme == "content") { "分享文件地址不受支持" }
        val archive = File(dir, "archive.zip")
        context.contentResolver.openInputStream(uri).use { input ->
            requireNotNull(input) { "无法读取 ZIP，请从微信重新分享" }
            archive.outputStream().use { out ->
                val buffer = ByteArray(16384); var total = 0
                while (true) { val n = input.read(buffer); if (n < 0) break; total += n
                    require(total <= WechatArchive.MAX_ARCHIVE) { "ZIP 不能超过 64 MiB" }; out.write(buffer, 0, n) }
            }
        }
        val imported = try { WechatArchive.read(archive, dir) } finally { archive.delete() }
        val files = imported.files.map { (assetId, file) ->
            val row = imported.document.messages.first { it.assetId == assetId }
            val type = when (row.kind) { "image" -> "image/jpeg"; "video" -> "video/mp4"; else -> "application/octet-stream" }
            PendingAttachment(if (row.kind == "image") "image" else "file", "微信聊天记录", row.filename, row.filename, type, file, null, null)
        }
        return ShareDraft(id, "", files, record = imported.document)
    }
}
