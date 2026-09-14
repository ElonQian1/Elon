package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class GroupMessageRevisionTest {
    @Test fun differencesPreserveUnicodeAndNewlines() {
        assertEquals("八" to "九", changedGroupRevisionText("八点\n🐲见", "九点\n🐲见"))
        assertEquals("🐲" to "🙂", changedGroupRevisionText("hi🐲", "hi🙂"))
        assertEquals("" to "", changedGroupRevisionText("同样文字", "同样文字"))
        assertEquals("" to "新增", changedGroupRevisionText("", "新增"))
        assertEquals("删除" to "", changedGroupRevisionText("删除", ""))
    }

    @Test fun delayedRefreshCannotRevertEditOrRecall() {
        val newest = ChatMessage("user", "第三版", id = "message", revision = 3, createdAtMs = 123L)
        val old = newest.copy(content = "第二版", revision = 2)
        val recalled = newest.copy(recalledAt = "recalled")
        assertEquals(newest, mergeGroupMessageRevisions(listOf(newest), listOf(old)).single())
        assertEquals(recalled, mergeGroupMessageRevisions(listOf(recalled), listOf(newest)).single())
        assertEquals(recalled, mergeGroupMessageRevisions(listOf(newest), listOf(recalled)).single())
        assertEquals(123L, mergeGroupMessageRevisions(listOf(newest), listOf(old)).single().createdAtMs)
    }
}
