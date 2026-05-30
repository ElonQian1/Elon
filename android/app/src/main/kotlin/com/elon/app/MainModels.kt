package com.elon.app

import java.io.File

data class AppConversation(
    val id: String,
    var title: String,
    var subtitle: String,
    var updatedAt: Long,
    var ended: Boolean = false,
    var codexThreadUri: String? = null,
    val messages: MutableList<ChatMessage>
)

data class AppProject(
    val id: String,
    var title: String,
    var subtitle: String,
    var updatedAt: Long,
    var stage: String = "待提交需求",
    var isJointProject: Boolean = false,
    var collaborationProjectId: String? = null,
    var collaborationJoinMode: String? = null,
    var activeConversationIndex: Int = 0,
    val events: MutableList<String> = mutableListOf(),
    val conversations: MutableList<AppConversation> = mutableListOf()
)

internal fun AppProject.isJointDevelopmentProject(): Boolean {
    val remoteId = collaborationProjectId?.trim().orEmpty()
    return isJointProject || remoteId.isNotBlank()
}

internal fun AppProject.projectSpaceId(): String {
    return collaborationProjectId?.trim()?.takeIf { it.isNotBlank() } ?: id
}

internal fun AppProject.projectJoinMode(): String {
    val mode = collaborationJoinMode?.trim()
    return when (mode) {
        "open", "approval", "invite" -> mode
        else -> "invite"
    }
}

internal fun AppProject.markJointDevelopment(remoteProjectId: String? = null, joinMode: String = "invite") {
    remoteProjectId?.trim()?.takeIf { it.isNotBlank() }?.let {
        collaborationProjectId = it
    }
    collaborationJoinMode = when (joinMode.trim()) {
        "open", "approval", "invite" -> joinMode.trim()
        else -> "invite"
    }
    isJointProject = true
    updatedAt = System.currentTimeMillis()
}

internal fun AppProject.markPersonalDevelopment() {
    isJointProject = false
    collaborationProjectId = null
    collaborationJoinMode = null
    updatedAt = System.currentTimeMillis()
}

data class ModelOption(
    val label: String,
    val agentName: String?,
    val provider: String = ""
)

data class ConversationTaskState(
    val traceId: String,
    val projectId: String,
    val conversationId: String,
    var payload: String,
    var isDevelopment: Boolean,
    var pendingReconnect: Boolean = false,
    var startedAt: Long = System.currentTimeMillis()
)

data class ServerVersionInfo(
    val versionName: String,
    val gitSha: String?
)

data class GitProjectStatus(
    val hasGit: Boolean,
    val origin: String?,
    val branch: String?,
    val remoteOk: Boolean?,
    val remoteMessage: String?,
    val deployKeyExists: Boolean,
    val publicKey: String?,
    val deployKeysUrl: String,
    val workflowTitle: String,
    val workflowSummary: String,
    val workflowSteps: List<String>,
    val codexMemory: String
)

data class PendingAttachment(
    val kind: String,
    val displayLabel: String = kind,
    val displayName: String,
    val fileName: String,
    val mimeType: String,
    val file: File,
    val imageWidth: Int? = null,
    val imageHeight: Int? = null,
    val durationSeconds: Int? = null,
    val transcription: String? = null
)

data class SendTarget(
    val projectId: String,
    val projectTitle: String,
    val conversationId: String,
    val conversationTitle: String
)

data class EvidenceEntry(
    val kind: String,
    val text: String
)

class TopAction(
    val title: String,
    val iconRes: Int,
    val action: () -> Unit
)
