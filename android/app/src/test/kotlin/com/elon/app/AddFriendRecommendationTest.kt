package com.elon.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class AddFriendRecommendationTest {
    @Test
    fun onlineStrangerIsVisibleWithoutBeingAnExistingFriend() {
        val item = parseRecommendation(JSONObject("""{"id":"usr_online","nickname":"Online User","is_online":true}"""))
        assertTrue(item.isOnline)
        assertFalse(item.alreadyFriend)
        assertEquals("Online User · 在线", item.displayName)
    }

    @Test
    fun offlineAndOlderResponsesNeverClaimToBeOnline() {
        for (presence in listOf("false", "null")) {
            val item = parseRecommendation(JSONObject("""{"id":"usr_offline","nickname":"Offline User","is_online":$presence}"""))
            assertFalse(item.isOnline)
            assertEquals("Offline User", item.displayName)
        }
        assertFalse(parseRecommendation(JSONObject("""{"id":"usr_old"}""")).isOnline)
    }

    @Test
    fun maskedAccountIsUsedForAccountAndMissingNickname() {
        for (hint in listOf("手机尾号 9650", "邮箱 p***m", "账号 u***r")) {
            val item = parseRecommendation(JSONObject().put("id", "usr_target").put("account_hint", hint))
            assertEquals(hint, item.account)
            assertEquals(hint, item.name)
        }
    }

    @Test
    fun maskedAccountDoesNotChangeIdentityOrAddState() {
        val item = parseRecommendation(JSONObject("""{"id":"usr_target","nickname":"Nickname","account_hint":"手机尾号 9650","already_friend":true,"mutual_friend_count":2}"""))
        assertEquals("usr_target", item.id)
        assertEquals("Nickname", item.name)
        assertEquals("手机尾号 9650", item.account)
        assertTrue(item.alreadyFriend)
        assertEquals(2, item.mutualFriendCount)
    }
}
