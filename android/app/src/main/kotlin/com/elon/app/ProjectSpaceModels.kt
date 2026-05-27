package com.elon.app

internal data class ProjectSpace(
    val project: ProjectSpaceSummary,
    val channels: List<ProjectChannel>,
    val members: List<ProjectMember>
)

internal data class ProjectSpaceSummary(
    val id: String,
    val name: String,
    val description: String?,
    val role: String,
    val memberCount: Int,
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
    val role: String,
    val joinedAt: String
)

internal data class ProjectChannelMessage(
    val id: String,
    val projectId: String,
    val channelId: String,
    val senderUserId: String?,
    val senderName: String?,
    val kind: String,
    val content: String,
    val taskId: String?,
    val createdAt: String,
    val outgoing: Boolean
)
