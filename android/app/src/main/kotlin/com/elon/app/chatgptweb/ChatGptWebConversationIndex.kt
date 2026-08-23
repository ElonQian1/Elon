package com.elon.app.chatgptweb

import java.time.LocalDate
import java.util.Locale

internal data class ChatGptWebConversationSection(
    val label: String,
    val conversations: List<ChatGptWebConversation>,
)

internal object ChatGptWebConversationIndex {
    private const val FALLBACK_GROUP = "历史会话"

    fun sections(values: List<ChatGptWebConversation>): List<ChatGptWebConversationSection> {
        val grouped = linkedMapOf<String, MutableList<ChatGptWebConversation>>()
        values.forEach { conversation ->
            val label = groupLabel(conversation.groupLabel) ?: FALLBACK_GROUP
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
        canonicalProjects(observed).forEach { canonical ->
            values.putIfAbsent(canonical.id, canonical)
        }
        conversations.forEach { conversation ->
            val sanitized = sanitize(conversation)
            val id = sanitized.projectId ?: return@forEach
            val path = sanitized.projectPath ?: "/g/$id/project"
            val title = metadataLabel(sanitized.projectTitle) ?: return@forEach
            values.putIfAbsent(id, ChatGptWebProject(id, title, path))
        }
        return values.values.toList()
    }

    fun mergeProjects(
        conversations: List<ChatGptWebConversation>,
        previous: List<ChatGptWebProject>,
        observed: List<ChatGptWebProject>,
        retainMissing: Boolean,
    ): List<ChatGptWebProject> {
        val previousById = canonicalProjects(previous).associateBy { it.id }
        val explicitReadableTitles = observed.mapNotNull { value ->
            val canonical = sanitizeProject(value) ?: return@mapNotNull null
            val hasReadableRoute = value.id.trim() != canonical.id || value.path.trim() != canonical.path
            if (hasReadableRoute) canonical.id to canonical.title else null
        }.toMap()
        val observedProjects = canonicalProjects(observed).map { next ->
            val old = previousById[next.id]
            next.copy(
                title = explicitReadableTitles[next.id]
                    ?: ChatGptWebProjectTitlePolicy.prefer(old?.title, next.title)
                    ?: next.title,
            )
        }
        return projects(
            conversations,
            observedProjects + if (retainMissing) previousById.values else emptyList(),
        )
    }

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

    fun observeCurrent(
        previous: List<ChatGptWebConversation>,
        snapshot: ChatGptWebSnapshot,
        activityDate: LocalDate,
        knownProjects: List<ChatGptWebProject> = emptyList(),
    ): List<ChatGptWebConversation> {
        val path = ChatGptWebConversationPath.fromUrl(snapshot.url) ?: return previous
        val identity = ChatGptWebConversationPath.identity(path) ?: return previous
        val existing = previous.firstOrNull { identityOf(it) == identity }
        val projectId = ChatGptWebConversationPath.projectId(path)
        val project = knownProjects.firstOrNull { it.id == projectId }
        val observedTitle = metadataLabel(snapshot.title)
            ?.takeUnless { it.lowercase(Locale.ROOT) in PLACEHOLDER_TITLES }
        val current = (existing ?: ChatGptWebConversation(
            id = identity,
            title = observedTitle ?: "新会话",
            path = path,
            active = true,
        )).copy(
            title = observedTitle ?: existing?.title ?: "新会话",
            path = path,
            active = true,
            projectId = projectId ?: existing?.projectId,
            projectTitle = project?.title ?: existing?.projectTitle,
            projectPath = projectId?.let { "/g/$it/project" } ?: existing?.projectPath,
            activityDates = existing?.activityDates.orEmpty() + activityDate.toString(),
        )
        return listOf(sanitize(current)) + previous
            .filterNot { identityOf(it) == identity }
            .map { sanitize(it.copy(active = false)) }
    }

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

    fun mergeOfficialHistory(
        previous: List<ChatGptWebConversation>,
        observed: List<ChatGptWebConversation>,
        collectionComplete: Boolean,
    ): List<ChatGptWebConversation> {
        if (!collectionComplete) return merge(previous, observed, retainMissing = true)

        val merged = merge(previous, observed, retainMissing = false)
        // A complete global sidebar scan is not authoritative for conversations owned by projects.
        val observedIdentities = observed.mapNotNullTo(linkedSetOf()) {
            ChatGptWebConversationPath.identity(it.path)
        }
        val cachedProjectConversations = collapse(previous)
            .filterKeys { it !in observedIdentities }
            .values
            .filter { it.projectId != null }
            .map(::sanitize)
        return merged + cachedProjectConversations
    }

    fun mergeProjectHistory(
        previous: List<ChatGptWebConversation>,
        observed: List<ChatGptWebConversation>,
        projectId: String,
        collectionComplete: Boolean,
    ): List<ChatGptWebConversation> {
        val canonicalId = ChatGptWebConversationPath.canonicalProjectId(projectId) ?: return previous
        val outsideProject = previous.filter { it.projectId != canonicalId }
        val cachedProject = previous.filter { it.projectId == canonicalId }
        val scopedObserved = observed
            .map(::sanitize)
            .filter { it.projectId == canonicalId }
        val projectValues = merge(
            cachedProject,
            scopedObserved,
            retainMissing = !collectionComplete,
        )
        return outsideProject + projectValues
    }

    fun sanitize(value: ChatGptWebConversation): ChatGptWebConversation {
        val projectId = ChatGptWebConversationPath.canonicalProjectId(value.projectId)
            ?: ChatGptWebConversationPath.projectId(value.path)
        return value.copy(
            groupLabel = groupLabel(value.groupLabel).orEmpty(),
            projectId = projectId,
            projectTitle = ChatGptWebProjectTitlePolicy.normalize(value.projectTitle),
            projectPath = projectId?.let { "/g/$it/project" },
        )
    }

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
            groupLabel = groupLabel(next.groupLabel)
                ?: groupLabel(previous.groupLabel)
                .orEmpty(),
            projectId = next.projectId ?: previous.projectId,
            projectTitle = ChatGptWebProjectTitlePolicy.prefer(
                previous.projectTitle,
                next.projectTitle,
            ),
            projectPath = next.projectPath ?: previous.projectPath,
            activityDates = previous.activityDates + next.activityDates,
            providerUrl = next.providerUrl ?: previous.providerUrl,
        )
    }

    private fun sanitizeProject(value: ChatGptWebProject): ChatGptWebProject? {
        val id = ChatGptWebConversationPath.canonicalProjectId(value.id)
            ?: ChatGptWebConversationPath.projectId(value.path)
            ?: return null
        val title = ChatGptWebProjectTitlePolicy.normalize(value.title) ?: return null
        return value.copy(id = id, title = title, path = "/g/$id/project")
    }

    internal fun canonicalProjects(values: List<ChatGptWebProject>): List<ChatGptWebProject> {
        val projects = linkedMapOf<String, Pair<Int, ChatGptWebProject>>()
        values.forEach { value ->
            val canonical = sanitizeProject(value) ?: return@forEach
            val specificity = (if (value.id.trim() != canonical.id) 2 else 0) +
                (if (value.path.trim() != canonical.path) 1 else 0)
            val current = projects[canonical.id]
            if (current == null || specificity > current.first) {
                projects[canonical.id] = specificity to canonical
            }
        }
        return projects.values.map { it.second }
    }

    private fun metadataLabel(value: String?): String? = value
        ?.trim()
        ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }

    private fun groupLabel(value: String?): String? = metadataLabel(value)
        ?.takeUnless { it.lowercase(Locale.ROOT) in NON_TEMPORAL_GROUP_LABELS }

    private val NON_TEMPORAL_GROUP_LABELS = setOf("pinned", "已置顶", "置顶")
    private val PLACEHOLDER_TITLES = setOf("chatgpt", "new chat", "新聊天", "新会话")

}
