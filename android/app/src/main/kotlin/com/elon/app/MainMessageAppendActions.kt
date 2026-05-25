package com.elon.app

import com.elon.app.databinding.ActivityMainBinding

internal class MainMessageAppendActions(
    private val binding: ActivityMainBinding,
    private val chatAdapter: () -> ChatAdapter,
    private val activeConversation: () -> AppConversation,
    private val workflowMessageCompactor: () -> MainWorkflowMessageCompactor,
    private val updateActiveConversationPreview: (ChatMessage) -> Unit,
    private val saveConversations: () -> Unit,
    private val workflowTerminalRoles: Set<String>
) {
    fun appendMessage(message: ChatMessage) {
        if (message.role in workflowTerminalRoles) {
            removeTransientWorkflowMessagesAfterLatestUser()
        }
        val adapter = chatAdapter()
        adapter.addMessage(message)
        updateActiveConversationPreview(message)
        binding.chatList.scrollToPosition(adapter.itemCount - 1)
    }

    private fun removeTransientWorkflowMessagesAfterLatestUser() {
        if (workflowMessageCompactor().removeTransientWorkflowMessagesAfterLatestUser(activeConversation().messages)) {
            chatAdapter().notifyDataSetChanged()
            saveConversations()
        }
    }
}
