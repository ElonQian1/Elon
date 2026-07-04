package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class ProjectConversationSyncBridge(
    private val projects: MutableList<AppProject>,
    private val activeProject: () -> AppProject,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val saveProjects: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val renderProjectList: () -> Unit
) {
    fun syncSummaries(conversations: List<ProjectMemberConversation>) {
        if (conversations.isEmpty()) return
        val project = localProjectForRemoteConversation(conversations.first().projectId) ?: return
        if (!mergeRemoteProjectConversations(project, conversations)) return
        saveProjects()
        renderConversationList()
        renderProjectList()
    }

    fun syncMessages(
        conversation: ProjectMemberConversation,
        messages: List<ProjectMemberConversationMessage>
    ): Int? {
        val project = localProjectForRemoteConversation(conversation.projectId) ?: return null
        val index = mergeRemoteProjectConversationMessages(project, conversation, messages)
        saveProjects()
        renderConversationList()
        renderProjectList()
        return index
    }

    private fun localProjectForRemoteConversation(projectId: String): AppProject? {
        val cleanProjectId = projectId.trim().takeIf { it.isNotBlank() } ?: return null
        val index = projects.indexOfFirst { project ->
            project.id == cleanProjectId || project.projectSpaceId() == cleanProjectId
        }
        if (index >= 0) {
            setActiveProjectIndex(index)
            return projects[index]
        }
        return activeProject().takeIf { project ->
            project.id == cleanProjectId || project.projectSpaceId() == cleanProjectId
        }
    }
}

internal fun projectConversationSyncBridge(
    state: MainActivityState,
    projectStateActions: MainProjectStateActions,
    homeListActions: MainHomeListActions,
    renderProjectList: () -> Unit = homeListActions::renderProjectList
) = ProjectConversationSyncBridge(
    projects = state.projects,
    activeProject = projectStateActions::activeProject,
    setActiveProjectIndex = { state.activeProjectIndex = it },
    saveProjects = projectStateActions::saveProjects,
    renderConversationList = homeListActions::renderConversationList,
    renderProjectList = renderProjectList
)

internal fun openRemotePersonalProjectConversation(
    activity: AppCompatActivity,
    http: OkHttpClient,
    serverUrl: String,
    conversation: ProjectMemberConversation,
    member: ProjectMember,
    space: ProjectSpace,
    syncMessages: (ProjectMemberConversation, List<ProjectMemberConversationMessage>) -> Int?,
    setPendingMemberBack: (ProjectMember) -> Unit,
    openPersonalAiChat: (Int) -> Unit
) {
    Toast.makeText(activity, "正在同步会话...", Toast.LENGTH_SHORT).show()
    thread(name = "project-personal-conversation-sync") {
        val result = runCatching {
            val messages = fetchProjectMemberConversationMessages(
                http = http,
                serverUrl = serverUrl,
                context = activity,
                projectId = space.project.id,
                memberUserId = member.userId,
                conversationId = conversation.id
            )
            syncMessages(conversation, messages)
        }
        activity.runOnUiThread {
            result
                .onSuccess { index ->
                    if (index != null && index >= 0) {
                        setPendingMemberBack(member)
                        openPersonalAiChat(index)
                    } else {
                        Toast.makeText(activity, "找不到该会话，可能已删除", Toast.LENGTH_SHORT).show()
                    }
                }
                .onFailure { error ->
                    Toast.makeText(activity, error.message ?: "同步会话失败", Toast.LENGTH_SHORT).show()
                }
        }
    }
}

internal fun mergeRemoteProjectConversations(
    project: AppProject,
    remoteConversations: List<ProjectMemberConversation>
): Boolean {
    var changed = false
    remoteConversations
        .filter { it.id.isNotBlank() }
        .forEach { remote ->
            val index = project.conversations.indexOfFirst { it.id == remote.id }
            if (index >= 0) {
                changed = updateConversationSummary(project.conversations[index], remote) || changed
            } else {
                project.conversations.add(remote.toAppConversation(emptyList()))
                changed = true
            }
        }
    val remoteCount = remoteConversations.count { it.id.isNotBlank() }
    if (remoteCount > 0 && (project.remoteConversationCount ?: 0) < remoteCount) {
        project.remoteConversationCount = remoteCount
        changed = true
    }
    if (changed) project.updatedAt = System.currentTimeMillis()
    return changed
}

internal fun mergeRemoteProjectConversationMessages(
    project: AppProject,
    remoteConversation: ProjectMemberConversation,
    remoteMessages: List<ProjectMemberConversationMessage>
): Int {
    val index = project.conversations.indexOfFirst { it.id == remoteConversation.id }
        .takeIf { it >= 0 }
        ?: run {
            project.conversations.add(remoteConversation.toAppConversation(remoteMessages))
            project.conversations.lastIndex
        }
    val local = project.conversations[index]
    updateConversationSummary(local, remoteConversation)

    val nextMessages = remoteMessages.map { it.toChatMessage() }
    if (nextMessages.isNotEmpty() && !local.hasPendingLocalMessage()) {
        if (!sameConversationMessages(local.messages, nextMessages)) {
            local.messages.clear()
            local.messages.addAll(nextMessages)
        }
    }

    project.remoteConversationCount = project.displayConversationCount().coerceAtLeast(project.conversations.size)
    project.updatedAt = maxOf(project.updatedAt, local.updatedAt)
    project.activeConversationIndex = index.coerceIn(0, project.conversations.lastIndex)
    return index
}

private fun ProjectMemberConversation.toAppConversation(
    remoteMessages: List<ProjectMemberConversationMessage>
): AppConversation {
    return AppConversation(
        id = id,
        title = remoteConversationTitle(this),
        subtitle = remoteConversationSubtitle(this),
        updatedAt = remoteConversationUpdatedAt(this),
        ended = status.equals("ended", ignoreCase = true) || status.equals("closed", ignoreCase = true),
        messages = remoteMessages.map { it.toChatMessage() }.toMutableList()
    )
}

private fun updateConversationSummary(
    local: AppConversation,
    remote: ProjectMemberConversation
): Boolean {
    var changed = false
    val title = remoteConversationTitle(remote)
    val subtitle = remoteConversationSubtitle(remote)
    val updatedAt = remoteConversationUpdatedAt(remote).coerceAtLeast(local.updatedAt)
    val ended = remote.status.equals("ended", ignoreCase = true) || remote.status.equals("closed", ignoreCase = true)
    if (!local.titleManuallyEdited && local.title != title) {
        local.title = title
        changed = true
    }
    if (local.subtitle != subtitle) {
        local.subtitle = subtitle
        changed = true
    }
    if (local.updatedAt != updatedAt) {
        local.updatedAt = updatedAt
        changed = true
    }
    if (local.ended != ended) {
        local.ended = ended
        changed = true
    }
    return changed
}

private fun remoteConversationTitle(conversation: ProjectMemberConversation): String {
    val raw = conversation.title?.trim()
        ?: conversation.lastMessage?.trim()
        ?: ""
    return summarize(raw.ifBlank { "项目频道" }, 24)
}

private fun remoteConversationSubtitle(conversation: ProjectMemberConversation): String {
    val parts = mutableListOf<String>()
    parts.add("${conversation.messageCount.coerceAtLeast(0)} 条消息")
    conversation.lastTaskStatus?.takeIf { it.isNotBlank() }?.let { status ->
        parts.add(
            when (status.lowercase()) {
                "running" -> "运行中"
                "done" -> "已完成"
                "failed", "error" -> "失败"
                "canceled", "cancelled" -> "已停止"
                else -> status
            }
        )
    }
    conversation.lastMessage?.trim()?.takeIf { it.isNotBlank() }?.let {
        parts.add(summarize(it, 40))
    }
    return parts.joinToString(" · ")
}

private fun remoteConversationUpdatedAt(conversation: ProjectMemberConversation): Long {
    return parseChatMessageCreatedAt(conversation.updatedAt)
        ?: parseChatMessageCreatedAt(conversation.lastMessageAt.orEmpty())
        ?: parseChatMessageCreatedAt(conversation.createdAt)
        ?: System.currentTimeMillis()
}

private fun AppConversation.hasPendingLocalMessage(): Boolean {
    return messages.any { !it.sendStatus.isNullOrBlank() }
}

private fun sameConversationMessages(
    current: List<ChatMessage>,
    next: List<ChatMessage>
): Boolean {
    if (current.size != next.size) return false
    return current.zip(next).all { (left, right) ->
        left.id == right.id &&
            left.role == right.role &&
            left.content == right.content &&
            left.senderLabel == right.senderLabel &&
            left.senderAvatarDataUrl == right.senderAvatarDataUrl &&
            left.createdAtMs == right.createdAtMs
    }
}
