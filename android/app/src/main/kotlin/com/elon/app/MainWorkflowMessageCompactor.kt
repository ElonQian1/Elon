package com.elon.app

internal class MainWorkflowMessageCompactor(
    private val staleWorkflowRoles: Set<String>,
    private val workflowHistoryStatusRoles: Set<String>,
    private val workflowTerminalRoles: Set<String>
) {
    fun removeLeakedAndRoutineWorkflowMessages(messages: MutableList<ChatMessage>) {
        messages.removeAll { message ->
            isLeakedPlatformPromptMessage(message.content) ||
                isTechnicalLeakMessage(message.content) ||
                (message.role in workflowHistoryStatusRoles && isRoutineWorkflowMessage(message.content))
        }
    }

    fun compactWorkflowStatusMessages(messages: MutableList<ChatMessage>) {
        if (messages.none { it.role in workflowHistoryStatusRoles }) return

        val compacted = mutableListOf<ChatMessage>()
        var pendingStatus: ChatMessage? = null

        for (message in messages) {
            when {
                message.role in workflowHistoryStatusRoles -> {
                    pendingStatus = if (message.role == "ai-cli-log") {
                        ChatMessage("ai-cli-log", genericFoldedCliLogSummary())
                    } else {
                        message
                    }
                }
                message.role in workflowTerminalRoles -> {
                    pendingStatus = null
                    compacted.add(message)
                }
                else -> {
                    pendingStatus?.let(compacted::add)
                    pendingStatus = null
                    compacted.add(message)
                }
            }
        }

        pendingStatus?.let(compacted::add)
        messages.clear()
        messages.addAll(compacted)
    }

    fun closeStaleWorkflowMessages(messages: MutableList<ChatMessage>) {
        val lastRole = messages.lastOrNull()?.role ?: return
        if (lastRole !in staleWorkflowRoles) return
        messages.removeAt(messages.lastIndex)
    }

    fun removeTransientWorkflowMessagesAfterLatestUser(messages: MutableList<ChatMessage>): Boolean {
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        if (latestUserIndex < 0) return false
        var removed = false
        for (index in messages.lastIndex downTo latestUserIndex + 1) {
            if (messages[index].role in staleWorkflowRoles) {
                messages.removeAt(index)
                removed = true
            }
        }
        return removed
    }
}
