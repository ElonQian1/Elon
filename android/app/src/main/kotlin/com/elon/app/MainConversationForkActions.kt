package com.elon.app

import com.elon.app.databinding.ActivityMainBinding

internal class MainConversationForkActions(
    private val binding: ActivityMainBinding,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val saveProjects: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val openConversation: (Int) -> Unit,
    private val renderProjectSpace: () -> Unit
) {
    fun forkForRunningInput(seedText: String): SendTarget {
        val project = activeProject()
        val source = activeConversation()
        val title = "分叉：${summarize(seedText, 14)}"
        val fork = newAppConversation(title, "分叉探索另一种方案").apply {
            messages.clear()
            messages.add(
                ChatMessage(
                    "ai-progress",
                    "已从「${source.title}」分叉，可以探索另一种方案。"
                )
            )
        }
        project.conversations.add(fork)
        setActiveConversationIndex(project.conversations.lastIndex)
        project.updatedAt = System.currentTimeMillis()
        project.subtitle = "${project.conversations.size} 个会话"
        saveProjects()
        renderConversationList()
        renderProjectSpace()
        openConversation(project.conversations.lastIndex)
        binding.topTitleText.text = fork.title
        return SendTarget(
            projectId = project.id,
            projectTitle = project.title,
            conversationId = fork.id,
            conversationTitle = fork.title
        )
    }
}
