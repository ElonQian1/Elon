package com.elon.app

import android.view.View
import com.elon.app.databinding.ActivityMainBinding

internal class MainConversationPreviewActions(
    private val binding: ActivityMainBinding,
    private val projects: () -> List<AppProject>,
    private val conversations: () -> MutableList<AppConversation>,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val activeProjectIndex: () -> Int,
    private val activeConversationIndex: () -> Int,
    private val chatAdapter: () -> ChatAdapter,
    private val conversationTaskKey: (String, String) -> String,
    private val workflowTerminalRoles: Set<String>,
    private val closeStaleWorkflowMessages: (MutableList<ChatMessage>) -> Unit,
    private val hasRunningTasks: () -> Boolean,
    private val saveConversations: () -> Unit,
    private val saveProjects: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val renderProjectList: () -> Unit
) {
    fun findConversationLocationByKey(key: String): Pair<Int, Int>? {
        projects().forEachIndexed { projectIndex, project ->
            project.conversations.forEachIndexed { conversationIndex, _ ->
                if (conversationTaskKey(project.id, project.conversations[conversationIndex].id) == key) {
                    return projectIndex to conversationIndex
                }
            }
        }
        return null
    }

    fun appendMessageToConversation(projectIndex: Int, conversationIndex: Int, message: ChatMessage) {
        val project = projects().getOrNull(projectIndex) ?: return
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        if (message.role in workflowTerminalRoles) {
            closeStaleWorkflowMessages(conversation.messages)
        }
        conversation.messages.add(message)
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        if (!conversation.ended) {
            conversation.subtitle = summarize(message.content, 30)
            project.subtitle = summarize(message.content, 34)
        }
        saveProjects()
        renderConversationList()
        if (projectIndex == activeProjectIndex() && conversationIndex == activeConversationIndex()) {
            chatAdapter().notifyDataSetChanged()
            binding.chatList.scrollToPosition(chatAdapter().itemCount - 1)
        }
    }

    fun updateFirstConversationStatus(text: String) {
        val conversations = conversations()
        if (conversations.isEmpty()) conversations.add(defaultAppConversation())
        if (conversations[0].ended) return
        conversations[0].subtitle = text
        conversations[0].updatedAt = System.currentTimeMillis()
        saveConversations()
        renderConversationList()
    }

    fun updateIdleReadyStatus() {
        if (!hasRunningTasks()) {
            updateFirstConversationStatus("已就绪 · 点击进入开发会话")
        }
    }

    fun updateActiveConversationPreview(message: ChatMessage) {
        val conversation = activeConversation()
        val project = activeProject()
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        when (message.role) {
            "user" -> updateUserPreview(conversation, project, message)
            "ai", "ai-intent", "ai-working", "ai-progress", "ai-tool", "ai-complete", "ai-stopped", "error" -> {
                if (!conversation.ended) {
                    conversation.subtitle = summarize(message.content, 30)
                    project.subtitle = summarize(message.content, 34)
                }
            }
        }
        saveConversations()
        renderConversationList()
        if (binding.projectPage.visibility == View.VISIBLE) renderProjectList()
    }

    private fun updateUserPreview(
        conversation: AppConversation,
        project: AppProject,
        message: ChatMessage
    ) {
        conversation.subtitle = summarize(message.content, 30)
        project.subtitle = summarize(message.content, 34)
        if (conversation.title.startsWith("新会话")) {
            conversation.title = summarize(message.content, 12)
            binding.topTitleText.text = conversation.title
        }
    }
}
