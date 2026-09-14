package com.elon.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class FriendSearchControllerTest {
    private class Fixture {
        val scheduled = mutableListOf<Runnable>()
        val queries = mutableListOf<String>()
        val replies = mutableListOf<(Result<JSONObject>) -> Unit>()
        var cancellations = 0
        var updates = 0
        val controller = FriendSearchController(
            schedule = { task, _ -> scheduled.add(task); Unit },
            unschedule = { scheduled.remove(it); Unit },
            lookup = { query, reply ->
                queries.add(query)
                replies.add(reply)
                val cancel: () -> Unit = { cancellations++ }
                cancel
            },
            publish = { updates++ }
        )

        fun runScheduled() {
            val tasks = scheduled.toList()
            scheduled.clear()
            tasks.forEach { it.run() }
        }

        fun found(id: String, self: Boolean = false, alreadyFriend: Boolean = false) =
            Result.success(JSONObject().put("found", true)
                .put("is_self", self).put("already_friend", alreadyFriend)
                .put("user", JSONObject().put("id", id).put("nickname", "Test User")))
    }

    @Test
    fun searchesRegisteredUserWithoutAnyRecommendationList() {
        val f = Fixture()
        f.controller.update(" 13900000052 ")
        assertTrue(f.controller.state.loading)
        assertTrue(f.queries.isEmpty())
        f.runScheduled()
        assertEquals(listOf("13900000052"), f.queries)
        f.replies.single()(f.found("usr_outside_first_50"))
        assertEquals("usr_outside_first_50", f.controller.state.users.singleOrNull()?.optString("id"))
        assertFalse(f.controller.state.loading)
    }

    @Test
    fun rapidTypingDebouncesAndKeyboardSearchRunsImmediately() {
        val f = Fixture()
        f.controller.update("13")
        f.controller.update("139")
        f.controller.update("13900000052", immediate = true)
        f.runScheduled()
        assertEquals(listOf("13900000052"), f.queries)
    }

    @Test
    fun staleSuccessAndErrorCannotReplaceCurrentResult() {
        val f = Fixture()
        f.controller.update("first", immediate = true)
        f.controller.update("second", immediate = true)
        f.replies[1](f.found("usr_second"))
        f.replies[0](f.found("usr_first"))
        f.replies[0](Result.failure(IOExceptionForTest()))
        assertEquals("usr_second", f.controller.state.users.singleOrNull()?.optString("id"))
        assertEquals(1, f.cancellations)
    }

    @Test
    fun clearAndCloseInvalidateEvenUncancelableRequests() {
        val f = Fixture()
        f.controller.update("first", immediate = true)
        f.controller.update("")
        f.replies[0](f.found("usr_first"))
        assertEquals(FriendSearchState(), f.controller.state)
        f.controller.update("second", immediate = true)
        f.controller.close()
        val updates = f.updates
        f.replies[1](f.found("usr_second"))
        f.controller.update("third")
        assertEquals(updates, f.updates)
        assertEquals(2, f.queries.size)
    }

    @Test
    fun reportsNotFoundDuplicateNameAndAllowsRetry() {
        val f = Fixture()
        f.controller.update("same name", immediate = true)
        f.replies[0](Result.failure(IllegalStateException("找到多个同名用户，请改用手机号")))
        assertTrue(f.controller.state.message.contains("多个同名用户"))
        assertTrue(f.controller.state.users.isEmpty())
        f.controller.update("13900000052", immediate = true)
        f.replies[1](Result.success(JSONObject().put("found", false)))
        assertTrue(f.controller.state.message.contains("未找到用户"))
        f.controller.update("13900000052", immediate = true)
        f.replies[2](f.found("usr_target"))
        f.controller.markAdded("usr_target")
        assertTrue(f.controller.state.users.single().optBoolean("already_friend"))
    }

    @Test
    fun selfAndExistingFriendAreExplicitlyMarked() {
        val f = Fixture()
        f.controller.update("self", immediate = true)
        f.replies[0](f.found("usr_self", self = true))
        assertTrue(f.controller.state.users.single().optBoolean("is_self"))
        assertTrue(f.controller.state.message.contains("不能添加自己"))
        f.controller.update("friend", immediate = true)
        f.replies[1](f.found("usr_friend", alreadyFriend = true))
        assertTrue(f.controller.state.users.single().optBoolean("already_friend"))
    }

    @Test
    fun sameNamesRemainSeparateAndOnlySelectedIdIsMarkedAdded() {
        val f = Fixture()
        f.controller.update("同名用户", immediate = true)
        f.replies[0](Result.success(JSONObject("""{"results":[
            {"id":"usr_first","nickname":"同名用户","account_hint":"手机尾号 1111","is_self":true},
            {"id":"usr_second","nickname":"同名用户","account_hint":"手机尾号 9650"},
            {"id":"usr_friend","nickname":"同名用户","already_friend":true}
        ],"has_more":false}""")))
        assertEquals(3, f.controller.state.users.size)
        assertTrue(f.controller.state.message.contains("多个同名"))
        assertEquals("手机尾号 9650", f.controller.state.users[1].optString("account_hint"))
        f.controller.markAdded("usr_second")
        assertFalse(f.controller.state.users[0].optBoolean("already_friend"))
        assertTrue(f.controller.state.users[1].optBoolean("already_friend"))
        assertTrue(f.controller.state.users[2].optBoolean("already_friend"))
        f.controller.update("new query", immediate = true)
        f.controller.markAdded("usr_second")
        assertTrue(f.controller.state.users.isEmpty())
    }

    @Test
    fun candidatesHandleTruncationEmptyAndMalformedResults() {
        val f = Fixture()
        listOf(
            """{"results":[{"id":"usr_one"}],"has_more":true}""" to "完整手机号",
            """{"results":[]}""" to "未找到用户",
            """{"results":[{"id":null}]}""" to "搜索结果异常"
        ).forEachIndexed { index, (body, expected) ->
            f.controller.update("test query", immediate = true)
            f.replies[index](Result.success(JSONObject(body)))
            assertTrue(f.controller.state.message.contains(expected))
        }
    }

    private class IOExceptionForTest : RuntimeException("old network error")
}
