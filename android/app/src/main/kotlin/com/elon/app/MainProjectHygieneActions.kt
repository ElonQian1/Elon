package com.elon.app

internal class MainProjectHygieneActions(
    private val timeText: () -> String,
    private val removeLeakedAndRoutineWorkflowMessages: (MutableList<ChatMessage>) -> Unit,
    private val compactWorkflowStatusMessages: (MutableList<ChatMessage>) -> Unit,
    private val closeStaleWorkflowMessages: (MutableList<ChatMessage>) -> Unit
) {
    fun normalizeProject(project: AppProject) {
        if (project.conversations.isEmpty()) project.conversations.add(defaultAppConversation())
        project.conversations.forEach { conversation ->
            if (conversation.messages.isEmpty()) conversation.messages.add(welcomeChatMessage())
            conversation.messages.forEach { message ->
                message.evidenceWorking = false
                message.sendStatus = null
            }
            compactCliTranscriptMessages(conversation.messages)
            sanitizeExistingCliLogMessages(conversation.messages)
            sanitizeExistingUserVisibleMessages(conversation.messages)
            removeLeakedAndRoutineWorkflowMessages(conversation.messages)
            compactWorkflowStatusMessages(conversation.messages)
            closeStaleWorkflowMessages(conversation.messages)
        }
        if (project.stage.isBlank()) project.stage = "待提交需求"
        if (project.subtitle.isBlank()) project.subtitle = "点击进入会话"
        compactCliProjectEvents(project.events)
        project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
    }

    private fun compactCliProjectEvents(events: MutableList<String>) {
        val cliCount = events.count { isCliProjectEvent(it) }
        if (cliCount == 0) return
        val compacted = events.filterNot { isCliProjectEvent(it) }.toMutableList()
        compacted.add(0, "${timeText()}  后台日志已归类：历史 ${cliCount} 条")
        while (compacted.size > 40) compacted.removeAt(compacted.size - 1)
        events.clear()
        events.addAll(compacted)
    }
}
