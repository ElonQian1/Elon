package com.elon.app

internal fun ArchiveProjectRecord.toStoreProject(ownerAccountFallback: String?): StoreProject {
    val systemKey = systemKey?.trim()
    return StoreProject(
        id = id,
        name = name,
        description = description,
        template = "android",
        ownerAccount = ownerAccount?.takeIf { it.isNotBlank() && it != "?" }
            ?: ownerAccountFallback?.takeIf { it.isNotBlank() }
            ?: "?",
        ownerUserId = ownerUserId.orEmpty(),
        memberCount = memberCount.coerceAtLeast(0),
        isPublic = isPublic,
        joinMode = joinMode,
        lastTaskStatus = lastTaskStatus,
        latestApkUrl = null,
        iconDataUrl = iconDataUrl,
        role = role,
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
