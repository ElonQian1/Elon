package com.elon.app

import android.content.Context
import com.elon.app.chatgptweb.ChatGptWebCanvasExportFormat
import com.elon.app.chatgptweb.ChatGptWebCanvasExportFormats
import com.elon.app.chatgptweb.ChatGptWebFileByteStorage
import com.elon.app.chatgptweb.ChatGptWebFileDownloadLease
import com.elon.app.chatgptweb.ChatGptWebFileDownloadPolicy
import java.util.UUID

internal object WebChatTextBlockExport {
    private val text = ChatGptWebCanvasExportFormat("txt", "txt", "text/plain", "文本 (.txt)")
    private val aliases = mapOf("js" to "javascript", "ts" to "typescript", "py" to "python", "sh" to "bash",
        "c++" to "cpp", "c#" to "csharp", "cs" to "csharp", "yml" to "yaml", "kt" to "kotlin", "rs" to "rust")

    fun formats(block: WebChatTextBlock): List<ChatGptWebCanvasExportFormat> {
        if (block.kind == "writing") return listOfNotNull(ChatGptWebCanvasExportFormats.find("document", "md"), text)
        val language = block.language.lowercase().let { aliases[it] ?: it }
        return listOfNotNull(ChatGptWebCanvasExportFormats.find("code/$language", "source"), text).distinctBy { it.extension }
    }

    fun name(stem: String, format: ChatGptWebCanvasExportFormat): String {
        val clean = ChatGptWebFileDownloadPolicy.safeName(stem.ifBlank { "document" }, 100)
        return if (clean.endsWith(".${format.extension}", ignoreCase = true)) clean else "$clean.${format.extension}"
    }

    // This is a user-requested local export, not a provider download or cloud-save receipt.
    fun save(context: Context, block: WebChatTextBlock, content: String, stem: String, format: ChatGptWebCanvasExportFormat) {
        require(block.complete && content.length <= WebChatTextBlock.MAX_CONTENT && format in formats(block))
        val ownership = ChatGptWebFileDownloadLease.Value(UUID.randomUUID().toString(), "local-text-block", 0,
            "", name(stem, format), format.mediaType, Long.MAX_VALUE)
        val target = ChatGptWebFileByteStorage.open(context, ownership) { }
        try {
            target.write(content.toByteArray(Charsets.UTF_8))
            target.publish()
        } catch (failure: Exception) {
            runCatching { target.discard() }
            throw failure
        }
    }
}
