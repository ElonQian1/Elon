package com.elon.app

internal data class WebChatLibraryEntry(
    val handle: String,
    val kind: String,
    val name: String,
    val mediaType: String,
    val sizeBytes: Long,
    val downloadHandle: String = "",
    val canRename: Boolean = false,
    val canTrash: Boolean = false,
    val canAttach: Boolean = false,
)

internal data class WebChatLibraryBreadcrumb(val handle: String, val name: String)

internal data class WebChatLibrarySnapshot(
    val requestId: String,
    val directoryHandle: String,
    val query: String,
    val breadcrumbs: List<WebChatLibraryBreadcrumb>,
    val items: List<WebChatLibraryEntry>,
    val hasMore: Boolean,
    val partial: Boolean,
    val stale: Boolean,
)

internal object WebChatLibraryPresentation {
    fun status(value: WebChatLibrarySnapshot?, busy: Boolean, failed: Boolean): String = when {
        failed && value != null -> "更新失败，保留已有文件"
        failed -> "文件库暂未读取成功，请重试"
        busy && value == null -> "正在读取文件库"
        busy -> "正在更新"
        value == null -> ""
        value.partial -> "${value.items.size} 项 · 部分内容尚未读取"
        value.items.isEmpty() && value.query.isNotBlank() -> "没有匹配的文件"
        value.items.isEmpty() -> "此文件夹为空"
        else -> "${value.items.size} 项"
    }

    fun subtitle(item: WebChatLibraryEntry): String = when {
        item.kind == "directory" -> "文件夹"
        item.sizeBytes >= 0 -> listOf(item.mediaType, "${item.sizeBytes} B").filter(String::isNotBlank).joinToString(" · ")
        else -> item.mediaType
    }
}
