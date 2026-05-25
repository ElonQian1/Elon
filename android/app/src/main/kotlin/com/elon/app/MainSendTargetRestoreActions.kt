package com.elon.app

import android.view.View
import com.elon.app.databinding.ActivityMainBinding

internal class MainSendTargetRestoreActions(
    private val binding: ActivityMainBinding,
    private val projects: MutableList<AppProject>,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val pauseCurrentWork: () -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val retryFailedAttachmentMessage: (ChatMessage) -> Unit,
    private val showChat: () -> Unit
) {
    fun restoreSendTarget(target: SendTarget): Boolean {
        val projectIndex = projects.indexOfFirst { it.id == target.projectId }
        if (projectIndex < 0) return false
        val project = projects[projectIndex]
        val conversationIndex = project.conversations.indexOfFirst { it.id == target.conversationId }
        if (conversationIndex < 0) return false

        setActiveProjectIndex(projectIndex)
        project.activeConversationIndex = conversationIndex
        val adapter = ChatAdapter(
            project.conversations[conversationIndex].messages,
            pauseCurrentWork,
            showMessageActions,
            retryFailedAttachmentMessage
        )
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        showChat()
        if (adapter.itemCount > 0) {
            binding.chatList.scrollToPosition(adapter.itemCount - 1)
        }
        return true
    }
}
