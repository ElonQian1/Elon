package com.elon.app

import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.ListView
import android.widget.TextView
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class GroupMembersDialogTest {
    private fun payload(count: Int): String = JSONObject().apply {
        put("members", JSONArray().apply {
            for (n in 1..count) put(JSONObject().put("id", "member-$n").put("display_name", "成员$n"))
        })
        put("ai_members", JSONArray().put(JSONObject().put("id", "virtual-ai").put("display_name", "EL")))
    }.toString()

    private fun client(body: () -> Pair<Int, String>) = OkHttpClient.Builder().addInterceptor { chain ->
        assertEquals("/api/me/groups/test-group/members", chain.request().url.encodedPath)
        val (code, text) = body()
        Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1).code(code)
            .message("fixture").body(text.toResponseBody("application/json".toMediaType())).build()
    }.build()

    private fun views(root: View): List<View> = listOf(root) + if (root is ViewGroup)
        (0 until root.childCount).flatMap { views(root.getChildAt(it)) } else emptyList()

    private fun waitFor(predicate: () -> Boolean) {
        repeat(300) {
            shadowOf(Looper.getMainLooper()).idle()
            if (predicate()) return
            Thread.sleep(10)
        }
        fail("Timed out waiting for roster state")
    }

    @Test fun fullRosterIncludesSelfAndLatestMemberWhileMentionPickerKeepsItsOwnContract() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val http = client { 200 to payload(16) }
        try {
            val directory = GroupMentionDirectory(host.get(), http, "https://fixture.invalid")
            val dialog = GroupMembersDialog(host.get(), directory).show("test-group")
            val all = views(dialog.window!!.decorView)
            val list = all.filterIsInstance<ListView>().single()
            waitFor { list.adapter.count == 16 }
            assertTrue((0 until list.adapter.count).map { list.adapter.getItem(it) }.contains("成员1"))
            assertTrue((0 until list.adapter.count).map { list.adapter.getItem(it) }.contains("成员16"))
            val search = all.filterIsInstance<EditText>().single()
            search.setText("成员16")
            assertEquals(1, list.adapter.count)
            search.setText("不存在")
            assertEquals(0, list.adapter.count)
            assertTrue(all.filterIsInstance<TextView>().any { it.text.contains("未找到匹配") })
            dialog.dismiss()
            val done = CountDownLatch(1)
            var mentions = emptyList<GroupMentionTarget>()
            directory.load("test-group", "member-1") { mentions = it.getOrThrow(); done.countDown() }
            assertTrue(done.await(3, TimeUnit.SECONDS))
            assertEquals(16, mentions.size)
            assertFalse(mentions.any { it.id == "member-1" })
            assertTrue(mentions.any { it.isAi && it.id == "virtual-ai" })
        } finally { http.dispatcher.executorService.shutdown(); host.pause().stop().destroy() }
    }

    @Test fun deniedRequestCanRetryAndReopeningLoadsFreshMembers() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val request = AtomicInteger(0)
        val http = client {
            when (request.incrementAndGet()) {
                1 -> 403 to "{\"error\":\"not a member\"}"
                2 -> 200 to payload(16)
                else -> 200 to payload(17)
            }
        }
        try {
            val opener = GroupMembersDialog(host.get(), GroupMentionDirectory(host.get(), http, "https://fixture.invalid"))
            var dialog = opener.show("test-group")
            var all = views(dialog.window!!.decorView)
            val retry = all.filterIsInstance<Button>().single { it.text == "重新加载" }
            waitFor { retry.visibility == View.VISIBLE }
            assertEquals(0, all.filterIsInstance<ListView>().single().adapter.count)
            retry.performClick()
            waitFor { all.filterIsInstance<ListView>().single().adapter.count == 16 }
            dialog.dismiss()
            dialog = opener.show("test-group")
            all = views(dialog.window!!.decorView)
            waitFor { all.filterIsInstance<ListView>().single().adapter.count == 17 }
            dialog.dismiss()
            assertEquals(3, request.get())
        } finally { http.dispatcher.executorService.shutdown(); host.pause().stop().destroy() }
    }
}
