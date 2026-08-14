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
            val label = metadataLabel(conversation.groupLabel) ?: FALLBACK_GROUP
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
            val title = metadataLabel(conversation.projectTitle) ?: return@forEach
            values.putIfAbsent(path, ChatGptWebProject(id, title, path))
        }
        return values.values.toList()
    }

    fun mergeProjects(
        conversations: List<ChatGptWebConversation>,
        previous: List<ChatGptWebProject>,
        observed: List<ChatGptWebProject>,
        retainMissing: Boolean,
    ): List<ChatGptWebProject> = projects(
        conversations,
        if (retainMissing) observed + previous else observed,
    )

    fun mergeObservedProjects(
        conversations: List<ChatGptWebConversation>,
        previous: List<ChatGptWebProject>,
        observed: List<ChatGptWebProject>,
    ): List<ChatGptWebProject> = mergeProjects(
        conversations = conversations,
        previous = previous,
        observed = observed,
        retainMissing = true,
    )

    fun activeOn(
        values: List<ChatGptWebConversation>,
        date: LocalDate,
    ): List<ChatGptWebConversation> = values.filter { date.toString() in it.activityDates }

    fun unassigned(values: List<ChatGptWebConversation>): List<ChatGptWebConversation> =
        values.filter { it.projectId == null }

    fun identityOf(value: ChatGptWebConversation): String =
        ChatGptWebConversationPath.identity(value.path) ?: value.id

    fun merge(
        previous: List<ChatGptWebConversation>,
        observed: List<ChatGptWebConversation>,
        retainMissing: Boolean = true,
    ): List<ChatGptWebConversation> {
        val previousByIdentity = collapse(previous)
        val observedByIdentity = collapse(observed)
        val merged = observedByIdentity.map { (identity, next) ->
            val old = previousByIdentity[identity] ?: return@map sanitize(next)
            combine(old, next).copy(active = next.active)
        }
        if (!retainMissing) return merged
        return merged + previousByIdentity
            .filterKeys { it !in observedByIdentity }
            .values
            .map(::sanitize)
    }

    fun sanitize(value: ChatGptWebConversation): ChatGptWebConversation = value.copy(
        groupLabel = metadataLabel(value.groupLabel).orEmpty(),
        projectTitle = metadataLabel(value.projectTitle),
    )

    private fun collapse(values: List<ChatGptWebConversation>): LinkedHashMap<String, ChatGptWebConversation> =
        linkedMapOf<String, ChatGptWebConversation>().apply {
            values.forEach { conversation ->
                val identity = ChatGptWebConversationPath.identity(conversation.path) ?: return@forEach
                this[identity] = this[identity]?.let { combine(it, conversation) } ?: sanitize(conversation)
            }
        }

    private fun combine(
        previous: ChatGptWebConversation,
        next: ChatGptWebConversation,
    ): ChatGptWebConversation {
        val nextHasProject = next.projectId != null
        val previousHasProject = previous.projectId != null
        return next.copy(
            path = if (nextHasProject || !previousHasProject) next.path else previous.path,
            active = previous.active || next.active,
            groupLabel = metadataLabel(next.groupLabel)
                ?: metadataLabel(previous.groupLabel)
                .orEmpty(),
            projectId = next.projectId ?: previous.projectId,
            projectTitle = metadataLabel(next.projectTitle)
                ?: metadataLabel(previous.projectTitle),
            projectPath = next.projectPath ?: previous.projectPath,
            activityDates = previous.activityDates + next.activityDates,
        )
    }

    private fun metadataLabel(value: String?): String? = value
        ?.trim()
        ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }

}
