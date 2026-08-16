package com.elon.app

import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.RecyclerView

internal class WebChatProductionTranscriptState(
    private val clock: () -> Long = System::currentTimeMillis,
) {
    private val timestamps = linkedMapOf<String, Long>()
    private var followLatestRequested = false

    fun timestampFor(id: String): Long = timestamps.getOrPut(id, clock)

    fun requestFollowLatest() {
        followLatestRequested = true
    }

    fun cancelFollowLatest() {
        followLatestRequested = false
    }

    fun consumeFollowLatestRequest(): Boolean = followLatestRequested.also {
        followLatestRequested = false
    }
}

internal class WebChatProductionTranscript(
    private val list: RecyclerView,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    onMessageLongPress: (android.view.View, ChatMessage) -> Unit,
    onMessageAction: (ChatMessage, WebChatMessageAction) -> Unit,
    onContentOpen: (ChatMessage, WebChatProductionContentPart) -> Unit,
) {
    private val state = WebChatProductionTranscriptState()
    private val messages = mutableListOf<ChatMessage>()
    private val adapter = ChatAdapter(messages, onMessageLongPress = onMessageLongPress).apply {
        onWebChatMessageAction = onMessageAction
        onWebChatContentOpen = onContentOpen
    }
    private val updater = WebChatProductionMessageListUpdater(messages, adapter)

    fun activate() {
        setChatAdapter(adapter)
        list.adapter = adapter
        if (messages.isNotEmpty()) list.jumpToLatestMessageBeforeNextDraw()
    }

    fun currentMessages(): List<ChatMessage> = messages.toList()

    fun hasMessages(): Boolean = messages.isNotEmpty()

    fun timestampFor(id: String): Long = state.timestampFor(id)

    fun requestFollowLatest() = state.requestFollowLatest()

    fun cancelFollowLatest() = state.cancelFollowLatest()

    fun indexOfMessageId(id: String): Int = messages.indexOfFirst { it.id == id }

    fun messageAt(index: Int): ChatMessage? = messages.getOrNull(index)

    fun submit(next: List<ChatMessage>, active: Boolean) {
        val followLatest = list.shouldFollowLatestWebChatMessage(
            state.consumeFollowLatestRequest(),
        )
        updater.submit(next, dispatchUpdates = active)
        if (active && followLatest && messages.isNotEmpty()) {
            list.jumpToLatestMessageBeforeNextDraw()
        }
    }

    fun showStatus(provider: WebChatProviderIdentity, content: String) {
        val id = "${provider.id.wireValue}:status"
        updater.submit(
            listOf(
                ChatMessage(
                    role = "friend",
                    content = content,
                    senderLabel = provider.displayName,
                    senderAvatarResId = provider.avatarResId,
                    id = id,
                    createdAtMs = timestampFor(id),
                ),
            ),
            dispatchUpdates = true,
        )
    }
}

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
