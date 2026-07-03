package com.elon.app

internal data class ProjectSpace(
    val project: ProjectSpaceSummary,
    val channels: List<ProjectChannel>,
    val members: List<ProjectMember>,
    val latestApkUrl: String?,
    val latestApkIdentity: String?,
    val latestApkUpdatedAt: String?,
    val galleryImages: List<String>,
    val landingPreviewImages: List<String>
)

internal data class ProjectSpaceSummary(
    val id: String,
    val name: String,
    val description: String?,
    val role: String,
    val joinMode: String,
    val memberCount: Int,
    val iconDataUrl: String?,
    val updatedAt: String
)

internal data class ProjectChannel(
    val id: String,
    val projectId: String,
    val name: String,
    val kind: String,
    val position: Int,
    val lastMessage: String?,
    val lastMessageAt: String?,
    val unreadCount: Int
)

internal data class ProjectMember(
    val userId: String,
    val account: String,
    val avatarDataUrl: String?,
    val role: String,
    val joinedAt: String
)

internal data class ProjectMemberConversation(
    val id: String,
    val projectId: String,
    val userId: String,
    val userAccount: String,
    val title: String?,
    val status: String,
    val isPublic: Boolean,
    val messageCount: Int,
    val taskCount: Int,
    val lastMessage: String?,
    val lastMessageRole: String?,
    val lastMessageAt: String?,
    val lastTaskStatus: String?,
    val createdAt: String,
    val updatedAt: String
)

internal data class ProjectMemberConversationMessage(
    val id: String,
    val projectId: String,
    val conversationId: String?,
    val taskId: String?,
    val userId: String?,
    val senderName: String?,
    val senderAvatarDataUrl: String?,
    val role: String,
    val content: String,
    val createdAt: String,
    val outgoing: Boolean
)

internal data class ProjectChannelMessage(
    val id: String,
    val projectId: String,
    val channelId: String,
    val senderUserId: String?,
    val senderName: String?,
    val senderAvatarDataUrl: String?,
    val replyToMessageId: String?,
    val kind: String,
    val content: String,
    val taskId: String?,
    val taskStatus: String?,
    val taskError: String?,
    val taskApkUrl: String?,
    val taskCodexThreadId: String?,
    val suggestionStatus: String?,
    val suggestionResolvedBy: String?,
    val suggestionResolvedByName: String?,
    val suggestionResolvedAt: String?,
    val createdAt: String,
    val outgoing: Boolean
)
