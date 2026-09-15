package com.elon.app

import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files

class SocialChatRecoveryTest {
    @Test fun snapshotsSurviveNewInstanceAndExpireWithoutCrashing() {
        val root = Files.createTempDirectory("social-cache-test").toFile()
        try {
            var now = 1000L
            val first = SocialChatSnapshotStore(root) { now }
            assertTrue(first.write("group:g", JSONArray("[{\"id\":\"m\",\"content\":\"缓存原文\",\"revision\":2}]")))
            assertEquals("缓存原文", SocialChatSnapshotStore(root) { now }.read("group:g")!!.getJSONObject(0).getString("content"))
            assertNull(first.read("friend:g"))
            now += SocialChatSnapshotStore.MAX_AGE_MS + 1
            assertNull(first.read("group:g"))
            root.listFiles()!!.first().writeText("{broken")
            assertNull(first.read("group:g"))
        } finally { root.deleteRecursively() }
    }

    @Test fun cacheIsBoundedAndCannotBeRecreatedByLateAccountWrite() {
        val root = Files.createTempDirectory("social-cache-test").toFile()
        try {
            val cache = SocialChatSnapshotStore(root)
            assertFalse(cache.write("late", JSONArray("[]")) { false })
            assertNull(cache.read("late"))
            repeat(70) { cache.write("chat-$it", JSONArray("[]")) }
            assertTrue(root.listFiles()!!.size <= 60)
            assertFalse(cache.write("huge", JSONArray().put("字".repeat(SocialChatSnapshotStore.MAX_ENTRY_BYTES))))
            cache.remove("chat-69"); assertNull(cache.read("chat-69"))
        } finally { root.deleteRecursively() }
    }

    @Test fun pendingDoesNotBlockIncomingAndEditRecallNeverRollBack() {
        val pending = ChatMessage("user", "待发", sendStatus = "发送中...")
        val failed = ChatMessage("user", "失败草稿", sendStatus = "失败")
        val edited = ChatMessage("friend", "第二版", id = "m", revision = 2)
        val remote = ChatMessage("friend", "第一版", id = "m")
        val fresh = ChatMessage("friend", "新消息", id = "new")
        val result = mergeSocialChatMessages(listOf(edited, pending, failed), listOf(remote, fresh))
        assertEquals(listOf(edited, fresh, pending, failed), result)
        val recalled = edited.copy(content = "", recalledAt = "now")
        assertEquals(recalled, mergeSocialChatMessages(listOf(recalled), listOf(edited)).single())
        val rows = protectSocialChatRows(listOf(edited), JSONArray("[{\"id\":\"m\",\"content\":\"第一版\",\"revision\":1}]"))
        assertEquals("第二版", rows.getJSONObject(0).getString("content"))
        assertEquals(2, rows.getJSONObject(0).getInt("revision"))
        val removed = protectSocialChatRows(listOf(recalled), rows).getJSONObject(0)
        assertEquals("now", removed.getString("recalled_at")); assertEquals(0, removed.getJSONArray("attachments").length())
    }

    @Test fun receiptAndRefreshRaceProducesOneConfirmedMessage() {
        val pending = ChatMessage("user", "同一条", sendStatus = "发送中...")
        val received = ChatMessage("user", "新版本", id = "server-id", revision = 2)
        val messages = mutableListOf(received, pending)
        completeSocialChatSend(messages, pending, received.copy(content = "旧版本", revision = 1))
        assertEquals(listOf(received), messages)
        val uncertain = ChatMessage("user", "未确认", sendStatus = "发送中...")
        completeSocialChatSend(mutableListOf(uncertain), uncertain, ChatMessage("user", "无回执 ID"))
        assertTrue(uncertain.sendStatus!!.contains("未确认"))
    }
}
