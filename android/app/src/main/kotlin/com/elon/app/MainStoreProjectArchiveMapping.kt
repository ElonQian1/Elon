package com.elon.app

internal fun ArchiveProjectRecord.toStoreProject(ownerAccountFallback: String?): StoreProject {
    val systemKey = systemKey?.trim()
    return StoreProject(
        id = id,
        name = name,
        description = description,
        template = "android",
        ownerAccount = if (!systemKey.isNullOrBlank()) {
            SYSTEM_ARCHIVE_OWNER_ACCOUNT
        } else {
            ownerAccount?.takeIf { it.isNotBlank() && it != "?" }
                ?: ownerAccountFallback?.takeIf { it.isNotBlank() }
                ?: "?"
        },
        ownerUserId = ownerUserId.orEmpty(),
        memberCount = memberCount.coerceAtLeast(0),
        isPublic = isPublic,
        joinMode = joinMode,
        lastTaskStatus = lastTaskStatus,
        latestApkUrl = null,
        iconDataUrl = iconDataUrl,
        role = role,
        projectOriginType = projectOriginType,
        projectOriginLabel = projectOriginLabel,
        remoteConversationCount = conversationCount,
        workspaceKind = workspaceKind,
        workspaceHealthLabel = workspaceStatus?.displayLabel(systemKey),
        workspaceHealthTone = workspaceStatus?.displayTone(),
        archiveEntryKey = conversationRoute?.entryKey,
        archiveConversationTitle = conversationRoute?.conversationTitle,
        memoryScopeType = conversationRoute?.memoryScopeType,
        memoryScopeId = conversationRoute?.memoryScopeId
    )
}
