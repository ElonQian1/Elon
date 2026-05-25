package com.elon.app

import java.io.File

data class AppConversation(
    val id: String,
    var title: String,
    var subtitle: String,
    var updatedAt: Long,
    var ended: Boolean = false,
    val messages: MutableList<ChatMessage>
)

data class AppProject(
    val id: String,
    var title: String,
    var subtitle: String,
    var updatedAt: Long,
    var stage: String = "待提交需求",
    var activeConversationIndex: Int = 0,
    val events: MutableList<String> = mutableListOf(),
    val conversations: MutableList<AppConversation> = mutableListOf()
)

data class ModelOption(
    val label: String,
    val agentName: String?
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
    val displayName: String,
    val fileName: String,
    val mimeType: String,
    val file: File
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
