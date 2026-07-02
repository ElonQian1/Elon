package com.elon.app

internal fun AppProject.isConversationWorking(
    conversation: AppConversation,
    isTaskRunning: (String, String) -> Boolean
): Boolean {
    if (conversation.ended) return false
    return projectTaskBadgeIds().any { projectId ->
        val id = cleanConversationProjectId(projectId) ?: return@any false
        isTaskRunning(id, conversation.id)
    }
}

internal fun AppProject.isConversationWorkingAt(
    conversationIndex: Int,
    isTaskRunning: (String, String) -> Boolean
): Boolean {
    val conversation = conversations.getOrNull(conversationIndex) ?: return false
    return isConversationWorking(conversation, isTaskRunning)
}

internal fun AppProject.preferredConversationIndex(
    isTaskRunning: (String, String) -> Boolean
): Int {
    if (conversations.isEmpty()) return 0
    return conversations.indices.maxWithOrNull(
        compareBy<Int> { index ->
            conversationWorkingSortKey(isConversationWorkingAt(index, isTaskRunning))
        }.thenBy { index ->
            conversationOpenSortKey(conversations[index].ended)
        }.thenBy { index ->
            conversations[index].updatedAt
        }
    ) ?: 0
}

internal fun conversationWorkingSortKey(working: Boolean): Int {
    return if (working) 1 else 0
}

internal fun conversationOpenSortKey(ended: Boolean): Int {
    return if (ended) 0 else 1
}

private fun cleanConversationProjectId(value: String?): String? {
    return value
        ?.trim()
        ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}
