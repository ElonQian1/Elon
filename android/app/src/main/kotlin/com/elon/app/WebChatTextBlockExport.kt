package com.elon.app

import android.content.Context
import android.net.Uri
import com.elon.app.chatgptweb.ChatGptWebCanvasExportFormat
import com.elon.app.chatgptweb.ChatGptWebCanvasExportFormats
import com.elon.app.chatgptweb.ChatGptWebFileByteStorage
import com.elon.app.chatgptweb.ChatGptWebFileDownloadLease
import com.elon.app.chatgptweb.ChatGptWebFileDownloadPolicy
import java.util.UUID
import java.nio.CharBuffer
import java.nio.charset.CodingErrorAction

internal object WebChatTextBlockExport {
    data class Result(val name: String, val mediaType: String, val uri: Uri?)
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

    fun bytes(content: String): ByteArray {
        require(content.length <= WebChatTextBlock.MAX_CONTENT)
        val encoded = Charsets.UTF_8.newEncoder().onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT).encode(CharBuffer.wrap(content))
        return ByteArray(encoded.remaining()).also { encoded.get(it) }
    }

    // This is a user-requested local export, not a provider download or cloud-save receipt.
    fun save(context: Context, block: WebChatTextBlock, content: String, stem: String,
        format: ChatGptWebCanvasExportFormat): Result {
        require(block.complete && content.length <= WebChatTextBlock.MAX_CONTENT && format in formats(block))
        val bytes = bytes(content)
        val ownership = ChatGptWebFileDownloadLease.Value(UUID.randomUUID().toString(), "local-text-block", 0,
            "", name(stem, format), format.mediaType, Long.MAX_VALUE)
        var savedUri: Uri? = null
        val target = ChatGptWebFileByteStorage.open(context, ownership) { savedUri = it }
        try {
            target.write(bytes)
            target.publish()
        } catch (failure: Exception) {
            runCatching { target.discard() }
            throw failure
        }
        return Result("elon-${ownership.id}-${ownership.name}", format.mediaType, savedUri)
    }
}
