package com.elon.app

import androidx.recyclerview.widget.DiffUtil

internal class WebChatProductionMessageListUpdater(
    private val messages: MutableList<ChatMessage>,
    private val adapter: ChatAdapter,
) {
    fun submit(next: List<ChatMessage>, dispatchUpdates: Boolean) {
        val previous = messages.toList()
        if (previous == next) return
        val diff = if (dispatchUpdates) {
            DiffUtil.calculateDiff(WebChatProductionMessageDiff(previous, next))
        } else {
            null
        }
        messages.clear()
        messages.addAll(next)
        diff?.dispatchUpdatesTo(adapter)
    }
}

internal class WebChatProductionMessageDiff(
    private val previous: List<ChatMessage>,
    private val next: List<ChatMessage>,
) : DiffUtil.Callback() {
    override fun getOldListSize(): Int = previous.size

    override fun getNewListSize(): Int = next.size

    override fun areItemsTheSame(oldItemPosition: Int, newItemPosition: Int): Boolean =
        WebChatProductionMessageDiffPolicy.areItemsTheSame(
            previous[oldItemPosition],
            next[newItemPosition],
        )

    override fun areContentsTheSame(oldItemPosition: Int, newItemPosition: Int): Boolean =
        previous[oldItemPosition] == next[newItemPosition]
}

internal object WebChatProductionMessageDiffPolicy {
    fun areItemsTheSame(previous: ChatMessage, next: ChatMessage): Boolean {
        val previousId = previous.id?.takeIf(String::isNotBlank) ?: return false
        return previousId == next.id
    }
}

internal object WebChatProductionScrollFollowPolicy {
    fun shouldFollow(force: Boolean, itemCount: Int, lastVisiblePosition: Int): Boolean {
        if (force || itemCount == 0) return true
        if (lastVisiblePosition < 0) return false
        return lastVisiblePosition >= itemCount - 1 - NEAR_END_ITEM_COUNT
    }

    private const val NEAR_END_ITEM_COUNT = 2
}
