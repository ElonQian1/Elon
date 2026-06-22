package com.elon.app

import android.content.Context

internal fun AppProject.toProjectHomeFriend(): AppFriend {
    val subtitle = projectHomeSubtitle()
    return AppFriend(
        id = projectHomeItemId(),
        name = projectHomeTitle(),
        account = subtitle,
        phone = null,
        avatarDataUrl = cleanProjectHomeText(iconDataUrl),
        friendSince = null,
        lastMessage = subtitle,
        lastMessageAt = updatedAt.takeIf { it > 0L },
        unreadCount = 0,
        isOnline = projectHomeStageIsRunning()
    )
}

internal fun AppProject.toProjectHomeGroup(
    context: Context,
    remoteMembers: List<AppGroupMember>?
): AppGroup {
    val members = remoteMembers?.takeIf { it.isNotEmpty() } ?: projectHomeFallbackMembers(context)
    return AppGroup(
        id = projectHomeItemId(),
        name = projectHomeTitle(),
        memberCount = projectHomeMemberCount(remoteMembers),
        members = members,
        createdAt = updatedAt.takeIf { it > 0L },
        lastMessage = projectHomeSubtitle(),
        lastMessageAt = updatedAt.takeIf { it > 0L },
        unreadCount = 0
    )
}

internal fun List<ProjectMember>.toProjectHomeGroupMembers(): List<AppGroupMember> {
    return mapNotNull { member ->
        val id = member.userId.trim()
        val name = cleanProjectHomeText(member.account) ?: cleanProjectHomeText(member.role) ?: "成员"
        if (id.isBlank() && name.isBlank()) return@mapNotNull null
        AppGroupMember(
            id = id,
            displayName = name,
            avatarDataUrl = cleanProjectHomeText(member.avatarDataUrl)
        )
    }
}

internal fun AppProject.projectHomeSearchValues(members: List<AppGroupMember>?): List<String?> {
    return listOf(
        projectHomeTitle(),
        projectHomeSubtitle(),
        projectKindLabel(),
        projectOriginLabel(),
        "${projectHomeMemberCount(members)} 位成员",
        members.orEmpty().joinToString(" ") { it.displayName },
        projectSpaceId()
    )
}

private fun AppProject.projectHomeFallbackMembers(context: Context): List<AppGroupMember> {
    val count = projectHomeMemberCount(null).coerceAtLeast(1)
    val visibleCount = count.coerceAtMost(PROJECT_HOME_MAX_AVATAR_MEMBERS)
    val selfName = AuthManager.displayName(context).takeIf { it.isNotBlank() && it != "未登录" } ?: "我"
    val selfAvatar = UserProfileStore.load(context).avatarDataUrl?.takeIf { it.isNotBlank() }
    val members = mutableListOf(
        AppGroupMember(
            id = AuthManager.effectiveUserId(context),
            displayName = selfName,
            avatarDataUrl = selfAvatar
        )
    )
    repeat(visibleCount - 1) { index ->
        members.add(
            AppGroupMember(
                id = "${projectHomeItemId()}:member:${index + 2}",
                displayName = "成员${index + 2}",
                avatarDataUrl = null
            )
        )
    }
    return members
}

private fun AppProject.projectHomeSubtitle(): String {
    return cleanProjectHomeText(projectCardIntroduction())
        ?: cleanProjectHomeText(subtitle)
        ?: if (isJointDevelopmentProject()) {
            "${projectHomeMemberCount(null)} 位成员"
        } else {
            "${projectKindLabel()} · ${displayConversationCount()} 个会话"
        }
}

private fun AppProject.projectHomeMemberCount(remoteMembers: List<AppGroupMember>?): Int {
    val remoteCount = remoteMembers?.size?.takeIf { it > 0 }
    val storedCount = memberCount?.coerceAtLeast(1)
    return when {
        remoteCount != null && storedCount != null -> maxOf(remoteCount, storedCount)
        remoteCount != null -> remoteCount
        storedCount != null -> storedCount
        else -> 1
    }
}

private fun AppProject.projectHomeItemId(): String {
    return "project:${projectSpaceId().ifBlank { id }}"
}

private fun AppProject.projectHomeTitle(): String {
    return cleanProjectHomeText(title) ?: "项目名称"
}

private fun AppProject.projectHomeStageIsRunning(): Boolean {
    val cleanStage = stage.trim()
    return cleanStage.equals("running", ignoreCase = true) || cleanStage == "运行中"
}

private fun cleanProjectHomeText(value: String?): String? {
    val text = value?.trim().orEmpty()
    return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

private const val PROJECT_HOME_MAX_AVATAR_MEMBERS = 9
