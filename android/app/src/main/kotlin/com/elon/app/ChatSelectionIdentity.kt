package com.elon.app

import java.util.IdentityHashMap
import java.util.UUID
import android.view.View
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView

/** Revision is captured at selection time, not silently advanced by a stream update. */
data class SelectedChatMessageIdentity(val identity: String, val revision: Long)

internal class ChatSelectionIdentity {
    private var objects = IdentityHashMap<ChatMessage, String>()
    private var aliases = mutableMapOf<String, String>()
    private val selected = linkedMapOf<String, SelectedChatMessageIdentity>()
    private val snapshots = mutableMapOf<String, ChatMessage>()

    fun key(message: ChatMessage): String {
        objects[message]?.let { return it }
        val alias = alias(message)
        val key = alias?.let(aliases::get) ?: alias ?: "local:${UUID.randomUUID()}"
        objects[message] = key
        if (alias != null) aliases[alias] = key
        return key
    }

    fun reconcile(messages: List<ChatMessage>) {
        // Keep object identity across server-ID assignment, and wire identity across remapping.
        messages.forEach { message ->
            objects[message]?.let { key -> alias(message)?.let { aliases[it] = key } }
        }
        val nextObjects = IdentityHashMap<ChatMessage, String>()
        val nextAliases = mutableMapOf<String, String>()
        messages.forEach { message ->
            val key = key(message)
            nextObjects[message] = key
            alias(message)?.let { nextAliases[it] = key }
        }
        objects = nextObjects
        aliases = nextAliases
        val available = messages.filter { it.role in ROLES && !it.isRecalled() }.map(::key).toSet()
        selected.keys.retainAll(available)
        snapshots.keys.retainAll(available)
        val ambiguous = messages.groupingBy(::key).eachCount().filterValues { it > 1 }.keys
        selected.keys.removeAll(ambiguous)
        snapshots.keys.removeAll(ambiguous)
    }

    fun select(message: ChatMessage) {
        if (isSelectable(message)) {
            val key = key(message)
            selected.putIfAbsent(key, SelectedChatMessageIdentity(key, message.revision))
            snapshots.putIfAbsent(key, message.copyForSharing())
        }
    }

    fun toggle(message: ChatMessage) {
        val key = key(message)
        if (selected.remove(key) == null) select(message) else snapshots.remove(key)
    }

    fun contains(message: ChatMessage): Boolean = key(message) in selected
    fun clear() { selected.clear(); snapshots.clear() }
    fun count(): Int = selected.size

    fun messagesInOrder(messages: List<ChatMessage>): List<ChatMessage> {
        reconcile(messages)
        return messages.filter { contains(it) }
            .mapNotNull { snapshots[key(it)]?.copyForSharing() }
    }

    fun metadataInOrder(messages: List<ChatMessage>): List<SelectedChatMessageIdentity> {
        reconcile(messages)
        return messages.mapNotNull { selected[key(it)] }
    }

    private fun alias(message: ChatMessage): String? {
        val provider = message.webChatMessage?.providerWireValue.orEmpty()
        val id = message.id?.takeIf(String::isNotBlank)
        // ChatMessage.id is already provider-qualified for Web AI; optional render metadata may appear later.
        if (id != null) return "message:$id"
        message.webChatMessage?.sourceMessageId?.takeIf(String::isNotBlank)?.let {
            return "source:${provider.length}:$provider:$it"
        }
        return message.streamId?.takeIf(String::isNotBlank)?.let { "stream:$it" }
    }

    companion object {
        private val ROLES = setOf("user", "ai", "assistant", "ai-intent", "ai-complete", "friend")
        fun isSelectable(message: ChatMessage): Boolean =
            message.role in ROLES &&
                message.id?.substringAfterLast(':') !in setOf("pending_user", "streaming") &&
                !message.isRecalled() && (message.content.isNotBlank() ||
                !message.attachments.isNullOrEmpty() ||
                message.webChatMessage?.contentParts?.isNotEmpty() == true)
    }
}

/** Detach mutable message/list state before asynchronous sharing or read-only rendering. */
internal fun ChatMessage.copyForSharing(): ChatMessage = copy(
    attachments = attachments?.map { it.copy(annotations = it.annotations.toList()) },
    webChatMessage = webChatMessage?.let { metadata ->
        metadata.copy(actions = metadata.actions.toSet(), contentParts = metadata.contentParts.map { part ->
            part.copy(richCard = part.richCard?.let { card ->
                card.copy(periods = card.periods.toList(), metrics = card.metrics.toList(),
                    series = card.series.toList(), points = card.points.map { it.copy(values = it.values.toList()) })
            })
        })
    },
)

internal fun bindChatSelectionContent(container: ViewGroup?, click: View.OnClickListener?) {
    container ?: return
    for (index in 0 until container.childCount) {
        val child = container.getChildAt(index)
        child.setOnClickListener(click)
        child.setOnLongClickListener(null)
        if (child is android.widget.TextView) child.movementMethod = null
        if (child is ViewGroup) bindChatSelectionContent(child, click)
    }
}

internal fun bindChatSelectionLongPress(container: ViewGroup?, listener: View.OnLongClickListener?) {
    container ?: return
    for (index in 0 until container.childCount) {
        val child = container.getChildAt(index)
        // Keep existing voice-specific long-press actions.
        if (!child.isLongClickable) child.setOnLongClickListener(listener)
        if (child is ViewGroup) bindChatSelectionLongPress(child, listener)
    }
}

internal fun observeChatSelectionChanges(adapter: RecyclerView.Adapter<*>, changed: () -> Unit) {
    adapter.registerAdapterDataObserver(object : RecyclerView.AdapterDataObserver() {
        override fun onChanged() = changed()
        override fun onItemRangeChanged(start: Int, count: Int) = changed()
        override fun onItemRangeChanged(start: Int, count: Int, payload: Any?) = changed()
        override fun onItemRangeInserted(start: Int, count: Int) = changed()
        override fun onItemRangeRemoved(start: Int, count: Int) = changed()
        override fun onItemRangeMoved(from: Int, to: Int, count: Int) = changed()
    })
}

internal fun bindChatSelectionMode(holder: ChatAdapter.VH, clickListener: View.OnClickListener?) {
    holder.itemView.setOnClickListener(clickListener)
    holder.bubble?.setOnClickListener(clickListener)
    holder.text.setOnClickListener(null)
    holder.text.movementMethod = null
    holder.text.isClickable = false
    holder.itemView.setOnLongClickListener(null)
    holder.bubble?.setOnLongClickListener(null)
    holder.text.setOnLongClickListener(null)
    holder.itemView.isLongClickable = false
    holder.bubble?.isLongClickable = false
    holder.text.isLongClickable = false
    bindChatSelectionContent(holder.attachmentList, clickListener)
    bindChatSelectionContent(holder.webChatPartList, clickListener)
    holder.attachmentList?.tag = null
}

internal fun bindChatSelectionVisual(holder: ChatAdapter.VH, selectionMode: Boolean, canSelect: Boolean, selected: Boolean) {
    holder.itemView.setBackgroundColor(android.graphics.Color.TRANSPARENT)
    holder.itemView.alpha = if (selectionMode && !canSelect) 0.62f else 1f
    holder.bubble?.alpha = 1f
    holder.selectionCheck?.visibility = if (selectionMode && canSelect) View.VISIBLE else View.GONE
    holder.selectionCheck?.text = if (selected) "✓" else ""
    holder.selectionCheck?.setBackgroundResource(
        if (selected) R.drawable.bg_message_selection_on else R.drawable.bg_message_selection_off)
    holder.itemView.contentDescription = if (selectionMode && canSelect) {
        if (selected) "已选中消息，点击取消选择" else "未选中消息，点击选择"
    } else null
}
