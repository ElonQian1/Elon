package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject

internal class WebChatConversationProjectMoveDestinationFallback(
    private val ui: WebChatConversationProjectMoveUi,
    private val recoveryStore: WebChatConversationProjectMoveRecoveryStore,
    private val openOfficialFallback: () -> Unit,
    private val isCurrent: (Int) -> Boolean,
    private val onCancelled: () -> Unit,
    private val onPrepared: (
        ChatGptWebConversation,
        ChatGptWebProject,
        WebChatConsumerPort,
        Int,
    ) -> Unit,
    private val onPrepareFailed: (ChatGptWebConversation, ChatGptWebProject, Int) -> Unit,
) {
    private var shown = false

    fun reset() {
        shown = false
    }

    fun show(
        index: ChatGptWebConversationIndexState,
        conversation: ChatGptWebConversation,
        port: WebChatConsumerPort,
        epoch: Int,
    ): Boolean {
        if (shown) return false
        val destinations = WebChatConversationProjectMovePolicy.officialDestinations(
            index,
            conversation,
            port.state(),
        )
        if (destinations.isEmpty()) return false
        shown = true
        ui.showDestinationPicker(
            destinations = destinations,
            onCancelled = {
                if (isCurrent(epoch)) onCancelled()
            },
            onSelected = { selected ->
                if (!isCurrent(epoch)) return@showDestinationPicker
                recoveryStore.clear()
                if (recoveryStore.prepare(conversation, selected) == null) {
                    onPrepareFailed(conversation, selected, epoch)
                } else {
                    onPrepared(conversation, selected, port, epoch)
                }
            },
            openOfficialFallback = openOfficialFallback,
        )
        return true
    }
}
