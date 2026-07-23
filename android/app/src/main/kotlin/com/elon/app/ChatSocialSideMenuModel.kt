package com.elon.app

internal enum class SocialSidebarConversationType {
    FRIEND,
    GROUP
}

internal data class SocialSidebarConversationKey(
    val type: SocialSidebarConversationType,
    val id: String
)

internal data class SocialSidebarTimelineItem(
    val key: SocialSidebarConversationKey,
    val name: String,
    val avatarDataUrl: String?,
    val summary: String,
    val lastReceivedAt: Long,
    val unreadCount: Int,
    val message: ChatMessage? = null
)

internal enum class SocialSidebarContentType {
    ALL,
    MEDIA,
    TEXT,
    LINK,
    NOTE,
    FILE
}

internal fun buildSocialSidebarTimeline(
    friends: List<AppFriend>,
    groups: List<AppGroup>,
    activeFriendId: String?,
    activeGroupId: String?
): List<SocialSidebarTimelineItem> {
    val friendItems = friends.asSequence()
        .filter { it.id.isNotBlank() && it.id != activeFriendId }
        .mapNotNull { friend ->
            val time = friend.lastReceivedAt ?: return@mapNotNull null
            SocialSidebarTimelineItem(
                key = SocialSidebarConversationKey(SocialSidebarConversationType.FRIEND, friend.id),
                name = friend.name,
                avatarDataUrl = friend.avatarDataUrl,
                summary = friend.lastReceivedMessage.orEmpty().ifBlank { "收到一条新消息" },
                lastReceivedAt = time,
                unreadCount = friend.unreadCount.coerceAtLeast(0)
            )
        }
    val groupItems = groups.asSequence()
        .filter { it.id.isNotBlank() && it.id != activeGroupId }
        .mapNotNull { group ->
            val time = group.lastReceivedAt ?: return@mapNotNull null
            SocialSidebarTimelineItem(
                key = SocialSidebarConversationKey(SocialSidebarConversationType.GROUP, group.id),
                name = group.name,
                avatarDataUrl = group.members.firstOrNull()?.avatarDataUrl,
                summary = group.lastReceivedMessage.orEmpty().ifBlank { "收到一条群消息" },
                lastReceivedAt = time,
                unreadCount = group.unreadCount.coerceAtLeast(0)
            )
        }
    return (friendItems + groupItems)
        .distinctBy { it.key }
        .sortedWith(compareByDescending<SocialSidebarTimelineItem> { it.lastReceivedAt }.thenBy { it.name })
        .toList()
}

internal fun socialSidebarContentType(
    summary: String,
    attachments: List<ChatAttachment> = emptyList()
): SocialSidebarContentType {
    if (attachments.any { attachment ->
            attachment.isImage() ||
                attachment.kind == "video" ||
                attachment.mimeType.orEmpty().startsWith("video/")
        }
    ) {
        return SocialSidebarContentType.MEDIA
    }
    if (attachments.isNotEmpty()) return SocialSidebarContentType.FILE
    val text = summary.trim()
    return when {
        text.contains("【图片】") || text.contains("【视频】") ||
            text.contains("[图片]") || text.contains("[视频]") -> SocialSidebarContentType.MEDIA
        URL_PATTERN.containsMatchIn(text) -> SocialSidebarContentType.LINK
        text.startsWith("【笔记】") || text.startsWith("[笔记]") -> SocialSidebarContentType.NOTE
        text.startsWith("【文件】") || text.startsWith("[文件]") ||
            text.startsWith("【附件】") || text.startsWith("[附件]") -> SocialSidebarContentType.FILE
        else -> SocialSidebarContentType.TEXT
    }
}

internal fun SocialSidebarTimelineItem.matchesSocialSidebarFilter(
    filter: SocialSidebarContentType
): Boolean {
    if (filter == SocialSidebarContentType.ALL) return true
    val attachments = message?.attachments.orEmpty()
    return socialSidebarContentType(message?.content ?: summary, attachments) == filter
}

private val URL_PATTERN = Regex("""(?i)\b(?:https?://|www\.)\S+""")
