package com.elon.app.chatrecords

internal object ChatRecordPresentation {
    private val markers = mapOf("link" to "链接", "channels" to "视频号", "image" to "图片", "video" to "视频", "audio" to "语音", "file" to "文件", "forward" to "聊天记录")
    data class Card(val url: String, val title: String = "", val site: String = "")
    fun text(row: RecordRow, cards: List<Card> = emptyList()): String {
        var value = row.text.trim()
        val prefix = markers[row.kind]?.let { "[$it]" }
        val exported = prefix != null && value.startsWith(prefix)
        if (exported) value = value.removePrefix(prefix!!).trim()
        if (exported && row.filename.isNotBlank() && value == row.filename) return ""
        cards.forEach { value = value.replace(it.url, "") }
        return value.lines().filterNot { line -> exported && cards.any { it.site != "视频号" && it.title.isNotBlank() && line.trim() == it.title.trim() } }.joinToString("\n").trim()
    }
    fun initial(sender: String): String = sender.trim().ifBlank { "未知发送者" }.let { String(Character.toChars(it.codePointAt(0))) }
    fun color(sender: String): String {
        val colors = listOf("#356859", "#65528a", "#376586", "#855040", "#536b32", "#80546f")
        val hash = sender.trim().ifBlank { "未知发送者" }.hashCode().toLong() and 0xffffffffL
        return colors[(hash % colors.size).toInt()]
    }
    fun duration(milliseconds: Long): String = if (milliseconds <= 0) "" else "${milliseconds / 60000}:${(milliseconds / 1000 % 60).toString().padStart(2, '0')}"
}
