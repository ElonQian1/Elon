package com.elon.app.chatrecords

/** The export is a display format, not an account-identity or timestamp protocol. */
internal object WechatTextParser {
    private val date = Regex("\\d{4}年\\d{1,2}月\\d{1,2}日[ \\t]+\\d{1,2}:\\d{2}(?::\\d{2})?")
    private val marker = Regex("^\\[([^]]+)](?:[ \\t]*(.*))?$")
    private data class Header(val line: Int, val indent: Int, val sender: String, val time: String)
    fun parse(raw: String, title: String = "微信聊天记录"): ChatRecordDocument {
        require(raw.toByteArray(Charsets.UTF_8).size <= 1024 * 1024) { "聊天记录文本超过 1 MiB" }
        val lines = raw.removePrefix("\uFEFF").replace("\r\n", "\n").replace('\r', '\n').split('\n')
        fun indent(s: String): Int = s.takeWhile { it == ' ' || it == '\t' }.fold(0) { n, c -> n + if (c == '\t') 4 else 1 }
        val headers = lines.indices.mapNotNull { i ->
            val label = lines[i].trimStart()
            val next = lines.getOrNull(i + 1) ?: return@mapNotNull null
            if (label.startsWith("·") && label.length > 1 && date.matches(next.trim()) && indent(next) == indent(lines[i])) {
                val sender = label.drop(1).trim()
                require(sender.codePointCount(0, sender.length) <= 200) { "发送者名称过长" }
                Header(i, indent(lines[i]), sender, next.trim())
            } else null
        }
        if (headers.isEmpty()) return ChatRecordDocument(title, raw,
            listOf(RecordRow("m1", null, "未知发送者", "", "unknown", raw.take(raw.offsetByCodePoints(0, minOf(60_000, raw.codePointCount(0, raw.length)))))),
            listOf("无法识别消息边界，保留原始文本；请核对导出格式。"))
        require(headers.size <= 2000) { "每次最多导入 2000 条记录" }
        val warnings = mutableListOf<String>()
        if (lines.take(headers.first().line).any { it.isNotBlank() }) warnings += "导出文件的前置信息保留在原文中。"
        val stack = mutableListOf<Pair<Int, RecordRow>>()
        val rows = mutableListOf<RecordRow>()
        headers.forEachIndexed { index, h ->
            while (stack.isNotEmpty() && stack.last().first >= h.indent) stack.removeAt(stack.lastIndex)
            require(stack.size <= 8) { "聊天记录嵌套超过 8 层" }
            val parent = stack.lastOrNull()?.second
            require(parent == null || parent.kind == "forward") { "聊天记录缩进不完整，无法安全确定转发层级" }
            val body = lines.subList(h.line + 2, headers.getOrNull(index + 1)?.line ?: lines.size)
                .joinToString("\n") { line -> line.dropWhileAtMost(h.indent) }.trim('\n')
            require(body.toByteArray(Charsets.UTF_8).size <= 256 * 1024) { "单条消息过大" }
            val first = body.lineSequence().firstOrNull { it.isNotBlank() }.orEmpty()
            val match = marker.matchEntire(first.trim())
            val kind = when (match?.groupValues?.get(1)) {
                "聊天记录" -> "forward"; "图片" -> "image"; "视频" -> "video"; "语音" -> "audio"
                "文件" -> "file"; "链接" -> "link"; "视频号" -> "channels"; null -> "text"; else -> "unknown"
            }
            val filename = if (kind in setOf("image", "video", "audio", "file")) match?.groupValues?.get(2).orEmpty().trim() else ""
            require(filename.length <= 240) { "附件名称过长" }
            val row = RecordRow("m${index + 1}", parent?.id, h.sender, h.time, kind, body, filename)
            rows += row; stack += h.indent to row
        }
        return ChatRecordDocument(title, raw, rows, warnings)
    }
    private fun String.dropWhileAtMost(columns: Int): String {
        var index = 0; var removed = 0
        while (index < length && removed < columns && (this[index] == ' ' || this[index] == '\t')) {
            removed += if (this[index] == '\t') 4 else 1; index++
        }
        return substring(index)
    }
}
