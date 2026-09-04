package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject

internal class WebChatPrivateConversationProjectMoveCoordinator(
    activity: AppCompatActivity,
    host: android.view.View,
    private val conversationIndex: () -> ChatGptWebConversationIndexState,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val startMutation: (ChatGptWebConversation, ChatGptWebProject) -> Unit,
    private val openOfficialFallback: (ChatGptWebConversation) -> Unit,
) {
    private val ui = WebChatConversationProjectMoveUi(activity, host)

    fun show(conversation: ChatGptWebConversation) {
        val destinations = WebChatConversationProjectMovePolicy.destinations(
            conversationIndex(),
            conversation,
        )
        if (destinations.isEmpty()) {
            ui.showNoDestinations(
                onRefresh = { refreshConversationIndex(null) },
                onOfficialFallback = { openOfficialFallback(conversation) },
            )
            return
        }
        ui.showDestinationPicker(
            destinations = destinations,
            onCancelled = {},
            onSelected = { destination -> startMutation(conversation, destination) },
            openOfficialFallback = { openOfficialFallback(conversation) },
        )
    }

    fun cancelPending() = ui.dismissAll()
}
