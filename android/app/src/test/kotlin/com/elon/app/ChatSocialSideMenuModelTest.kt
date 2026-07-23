package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatSocialSideMenuModelTest {
    @Test
    fun timelineExcludesActiveTargetsDeduplicatesAndSortsNewestFirst() {
        val friends = listOf(
            friend("active", 400L, 9),
            friend("older", 100L, 2),
            friend("newer", 300L, 7)
        )
        val groups = listOf(
            group("active-group", 500L, 8),
            group("middle", 200L, 4)
        )

        val items = buildSocialSidebarTimeline(
            friends = friends,
            groups = groups,
            activeFriendId = "active",
            activeGroupId = "active-group"
        )

        assertEquals(listOf("newer", "middle", "older"), items.map { it.key.id })
        assertEquals(listOf(7, 4, 2), items.map { it.unreadCount })
        assertEquals(items.map { it.key }.distinct(), items.map { it.key })
        assertFalse(items.any { it.key.id == "active" || it.key.id == "active-group" })
    }

    @Test
    fun refreshedConversationReplacesItsSingleNodeAndMovesToTop() {
        val before = buildSocialSidebarTimeline(
            friends = listOf(friend("a", 100L, 1), friend("b", 200L, 2)),
            groups = emptyList(),
            activeFriendId = null,
            activeGroupId = null
        )
        val after = buildSocialSidebarTimeline(
            friends = listOf(friend("a", 300L, 5), friend("b", 200L, 2)),
            groups = emptyList(),
            activeFriendId = null,
            activeGroupId = null
        )

        assertEquals(listOf("b", "a"), before.map { it.key.id })
        assertEquals(listOf("a", "b"), after.map { it.key.id })
        assertEquals(1, after.count { it.key.id == "a" })
        assertEquals(5, after.first().unreadCount)
    }

    @Test
    fun timelineUsesLatestReceivedMessageInsteadOfNewerOutgoingMessage() {
        val item = buildSocialSidebarTimeline(
            friends = listOf(
                friend("friend", receivedTime = 200L, unread = 1, conversationTime = 500L)
            ),
            groups = listOf(
                group("group", receivedTime = 300L, unread = 2, conversationTime = 600L)
            ),
            activeFriendId = null,
            activeGroupId = null
        )

        assertEquals(listOf("group", "friend"), item.map { it.key.id })
        assertEquals(listOf(300L, 200L), item.map { it.lastReceivedAt })
        assertEquals(listOf("received-group", "received-friend"), item.map { it.summary })
    }

    @Test
    fun filtersRecognizeMediaTextLinksNotesAndFiles() {
        assertEquals(SocialSidebarContentType.MEDIA, socialSidebarContentType("【图片】"))
        assertEquals(SocialSidebarContentType.MEDIA, socialSidebarContentType(
            "",
            listOf(ChatAttachment(kind = "video", mimeType = "video/mp4"))
        ))
        assertEquals(SocialSidebarContentType.LINK, socialSidebarContentType("https://elon.example"))
        assertEquals(SocialSidebarContentType.NOTE, socialSidebarContentType("【笔记】排期"))
        assertEquals(SocialSidebarContentType.FILE, socialSidebarContentType("【文件】需求.pdf"))
        assertEquals(SocialSidebarContentType.TEXT, socialSidebarContentType("普通消息"))
        assertTrue(
            SocialSidebarTimelineItem(
                SocialSidebarConversationKey(SocialSidebarConversationType.FRIEND, "f"),
                "好友",
                null,
                "www.example.com",
                1L,
                0
            ).matchesSocialSidebarFilter(SocialSidebarContentType.LINK)
        )
    }

    private fun friend(
        id: String,
        receivedTime: Long,
        unread: Int,
        conversationTime: Long = receivedTime
    ) = AppFriend(
        id = id,
        name = id,
        account = id,
        phone = null,
        avatarDataUrl = null,
        friendSince = null,
        lastMessage = "message-$id",
        lastMessageAt = conversationTime,
        unreadCount = unread,
        lastReceivedMessage = "received-$id",
        lastReceivedAt = receivedTime
    )

    private fun group(
        id: String,
        receivedTime: Long,
        unread: Int,
        conversationTime: Long = receivedTime
    ) = AppGroup(
        id = id,
        name = id,
        memberCount = 2,
        members = emptyList(),
        createdAt = receivedTime,
        lastMessage = "message-$id",
        lastMessageAt = conversationTime,
        unreadCount = unread,
        lastReceivedMessage = "received-$id",
        lastReceivedAt = receivedTime
    )
}
