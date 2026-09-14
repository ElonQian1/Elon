package com.elon.app

import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

internal data class GroupTextRevision(val revision: Long, val content: String, val createdAt: String)

internal fun changedGroupRevisionText(before: String, after: String): Pair<String, String> {
    val a = before.codePoints().toArray()
    val b = after.codePoints().toArray()
    var start = 0
    var end = 0
    while (start < a.size && start < b.size && a[start] == b[start]) start++
    while (end < a.size - start && end < b.size - start && a[a.size - end - 1] == b[b.size - end - 1]) end++
    return String(a, start, a.size - start - end) to String(b, start, b.size - start - end)
}

internal fun renderGroupRevisionHistory(container: LinearLayout, revisions: List<GroupTextRevision>) {
    container.removeAllViews()
    fun line(value: String, secondary: Boolean = false) = TextView(container.context).apply {
        text = value
        textSize = if (secondary) 13f else 15f
        setTextColor(ContextCompat.getColor(context, if (secondary) R.color.elon_text_secondary else R.color.elon_text_primary))
        setTextIsSelectable(true)
        val padding = (8 * resources.displayMetrics.density).toInt()
        setPadding(0, padding, 0, padding)
        container.addView(this)
    }
    revisions.forEachIndexed { index, version ->
        val time = runCatching { DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss").withZone(ZoneId.systemDefault()).format(Instant.parse(version.createdAt)) }.getOrDefault(version.createdAt)
        line("第 ${version.revision} 版${if (version.revision == 1L) " · 原始文字" else ""}\n$time", true)
        line(version.content)
        revisions.getOrNull(index + 1)?.let { previous ->
            val (removed, added) = changedGroupRevisionText(previous.content, version.content)
            val changes = line(listOfNotNull(removed.takeIf { it.isNotEmpty() }?.let { "删除：$it" }, added.takeIf { it.isNotEmpty() }?.let { "新增：$it" }).joinToString("\n"))
            changes.visibility = android.view.View.GONE
            android.widget.Button(container.context).apply {
                text = "与上一版相比"
                setOnClickListener { changes.visibility = if (changes.visibility == android.view.View.GONE) android.view.View.VISIBLE else android.view.View.GONE }
                container.addView(this)
            }
        }
    }
}
