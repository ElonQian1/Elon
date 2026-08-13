package com.elon.app.chatgptweb

import java.time.LocalDate

internal data class ChatGptWebConversationSection(
    val label: String,
    val conversations: List<ChatGptWebConversation>,
)

internal object ChatGptWebConversationIndex {
    private const val FALLBACK_GROUP = "历史会话"

    fun sections(values: List<ChatGptWebConversation>): List<ChatGptWebConversationSection> {
        val grouped = linkedMapOf<String, MutableList<ChatGptWebConversation>>()
        values.forEach { conversation ->
            val label = conversation.groupLabel.trim().ifBlank { FALLBACK_GROUP }
            grouped.getOrPut(label) { mutableListOf() }.add(conversation)
        }
        return grouped.map { (label, conversations) ->
            ChatGptWebConversationSection(label, conversations)
        }
    }

    fun projects(
        conversations: List<ChatGptWebConversation>,
        observed: List<ChatGptWebProject> = emptyList(),
    ): List<ChatGptWebProject> {
        val values = linkedMapOf<String, ChatGptWebProject>()
        observed.forEach { project -> values.putIfAbsent(project.path, project) }
        conversations.forEach { conversation ->
            val id = conversation.projectId ?: return@forEach
            val path = conversation.projectPath
                ?.let(ChatGptWebConversationPath::normalizeProject)
                ?: "/g/$id/project"
            val title = conversation.projectTitle?.trim().orEmpty()
            if (title.isBlank()) return@forEach
            values.putIfAbsent(path, ChatGptWebProject(id, title, path))
        }
        return values.values.toList()
    }

    fun activeOn(
        values: List<ChatGptWebConversation>,
        date: LocalDate,
    ): List<ChatGptWebConversation> = values.filter { date.toString() in it.activityDates }

    fun merge(
        previous: List<ChatGptWebConversation>,
        observed: List<ChatGptWebConversation>,
    ): List<ChatGptWebConversation> {
        val previousByPath = previous.associateBy(ChatGptWebConversation::path)
        val merged = observed.map { next ->
            val old = previousByPath[next.path] ?: return@map next
            next.copy(
                groupLabel = next.groupLabel.ifBlank { old.groupLabel },
                projectId = next.projectId ?: old.projectId,
                projectTitle = next.projectTitle ?: old.projectTitle,
                projectPath = next.projectPath ?: old.projectPath,
                activityDates = old.activityDates + next.activityDates,
            )
        }
        val observedPaths = observed.asSequence().map(ChatGptWebConversation::path).toSet()
        return merged + previous.filterNot { it.path in observedPaths }
    }
}
