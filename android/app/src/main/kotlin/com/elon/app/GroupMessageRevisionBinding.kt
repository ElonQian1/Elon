package com.elon.app

import android.view.View
import android.widget.TextView

internal fun bindGroupMessageRevision(holder: ChatAdapter.VH, message: ChatMessage, open: ((ChatMessage) -> Unit)?) {
    val bubble = holder.bubble ?: return
    val visible = open != null && message.revision > 1 && message.recalledAt.isNullOrBlank()
    var label = bubble.findViewWithTag<TextView>("group-message-revision")
    if (label == null && visible) {
        label = TextView(bubble.context).apply {
            tag = "group-message-revision"
            textSize = 12f
            minHeight = (48 * resources.displayMetrics.density).toInt()
            gravity = android.view.Gravity.CENTER_VERTICAL
        }
        bubble.addView(label)
    }
    label?.visibility = if (visible) View.VISIBLE else View.GONE
    label?.text = "已编辑 · ${message.revision - 1} 次"
    label?.setTextColor(holder.text.currentTextColor)
    label?.contentDescription = "已编辑 ${message.revision - 1} 次，查看修改记录"
    label?.setOnClickListener(if (visible) View.OnClickListener { open?.invoke(message) } else null)
}

internal fun mergeGroupMessageRevisions(current: List<ChatMessage>, incoming: List<ChatMessage>): List<ChatMessage> {
    val byId = current.filter { !it.id.isNullOrBlank() }.associateBy { it.id }
    return incoming.map { message ->
        val previous = byId[message.id]
        if (previous != null && message.recalledAt.isNullOrBlank() &&
            (!previous.recalledAt.isNullOrBlank() || previous.revision > message.revision)) previous else message
    }
}
