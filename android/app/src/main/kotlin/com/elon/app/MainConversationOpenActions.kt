package com.elon.app

import android.view.View
import com.elon.app.databinding.ActivityMainBinding

internal class MainConversationOpenActions(
    private val binding: ActivityMainBinding,
    private val projects: () -> List<AppProject>,
    private val conversations: () -> MutableList<AppConversation>,
    private val activeConversation: () -> AppConversation,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val pauseCurrentWork: () -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val retryFailedAttachmentMessage: (ChatMessage) -> Unit,
    private val showChat: (Boolean) -> Unit,
    private val showProjectChat: (Boolean) -> Unit,
    private val saveProjects: () -> Unit
) {
    fun openConversation(index: Int) {
        val currentConversations = conversations()
        if (currentConversations.isEmpty()) currentConversations.add(defaultAppConversation())
        setActiveConversationIndex(index.coerceIn(0, currentConversations.lastIndex))
        openActiveConversation(showChat)
    }

    fun openProject(index: Int) {
        if (index !in projects().indices) return
        setActiveProjectIndex(index)
        val currentConversations = conversations()
        if (currentConversations.isEmpty()) currentConversations.add(defaultAppConversation())
        val recentConversationIndex = currentConversations
            .indices
            .maxByOrNull { currentConversations[it].updatedAt }
            ?: 0
        setActiveConversationIndex(recentConversationIndex)
        saveProjects()
        openActiveConversation(showProjectChat)
    }

    private fun openActiveConversation(show: (Boolean) -> Unit) {
        val adapter = ChatAdapter(activeConversation().messages, pauseCurrentWork, showMessageActions, retryFailedAttachmentMessage)
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        show(true)
        if (adapter.itemCount > 0) {
            binding.chatList.scrollToPosition(adapter.itemCount - 1)
        }
    }
}
