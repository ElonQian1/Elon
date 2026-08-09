package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SocialAiIdentityTest {
    @Test
    fun resolvesBuiltInFriendBeforeRemoteFriendListLoads() {
        val friend = SocialAiIdentity.resolve(emptyList())

        assertEquals("usr_elon_ai", friend.id)
        assertEquals("一龙AI", friend.name)
        assertTrue(friend.isOnline)
        assertTrue(friend.isSocialAi())
    }

    @Test
    fun preservesRemoteFriendSummaryWhenAvailable() {
        val remote = SocialAiIdentity.builtInFriend().copy(
            lastMessage = "已同步的预览",
            unreadCount = 3,
        )

        assertEquals(remote, SocialAiIdentity.resolve(listOf(remote)))
    }
}
