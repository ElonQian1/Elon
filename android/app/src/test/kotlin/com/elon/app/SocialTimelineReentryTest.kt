package com.elon.app

import android.app.Application
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONArray
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ActivityController
import org.robolectric.annotation.Config
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Real social controllers + production binding; no replacement navigation/read implementation. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], application = Application::class)
class SocialTimelineReentryTest {
    private lateinit var lifecycle: ActivityController<AppCompatActivity>
    private lateinit var activity: AppCompatActivity
    private lateinit var binding: ActivityMainBinding
    private lateinit var group: MainGroupChatActions
    private lateinit var friend: MainFriendChatActions
    private val room = AppGroup("room", "测试群", 2, emptyList(), null, null, null, 0)
    private val peer = AppFriend("peer", "好友", "peer", null, null, null, null, null, 0)
    private val requests = CopyOnWriteArrayList<String>()
    @Volatile private var newest = 240
    @Volatile private var fail = false
    @Volatile private var heldSync: CountDownLatch? = null
    private val syncStarted = CountDownLatch(1)

    private fun page(start: Int, end: Int) = JSONObject().put("schema", "elon.message_timeline.v1")
        .put("messages", JSONArray((start until end).map { n -> JSONObject().put("id", "%04d".format(n))
            .put("created_at", "2026-10-02T01:00:00Z").put("content", "fixture $n")
            .put("sender_user_id", "peer").put("timeline_cursor", n.toString()) }))
        .put("removed_ids", JSONArray()).put("has_more", start > 0).put("sync", "checkpoint").toString()
    private fun pump(until: () -> Boolean) {
        repeat(250) { shadowOf(Looper.getMainLooper()).idle(); if (until()) return; Thread.sleep(10) }
        assertTrue("social controller did not reach expected state", until())
    }
    private fun rows() = (binding.chatList.adapter as ChatAdapter).currentMessagesForSharing()
    private fun views(view: View = binding.root): List<View> = listOf(view) +
        if (view is ViewGroup) (0 until view.childCount).flatMap { views(view.getChildAt(it)) } else emptyList()
    private fun button(label: String) = views().filterIsInstance<Button>().firstOrNull { it.text == label && it.isShown }
    private fun older() {
        val previous = rows().first().id
        assertNotNull("older navigation must be attached", button("加载更早消息"))
        button("加载更早消息")!!.performClick()
        pump { rows().first().id != previous }
    }

    @Before fun setup() {
        lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        activity = lifecycle.get(); activity.setTheme(R.style.Theme_ElonApp); lifecycle.setup()
        binding = ActivityMainBinding.inflate(activity.layoutInflater); activity.setContentView(binding.root)
        AuthManager.prefs(activity).edit().putString("auth_user_id", "reentry-fixture")
            .putString("auth_token", "synthetic").putString("auth_session_revision", "reentry")
            .putLong("auth_expires_at", 0).commit()
        SocialChatSnapshotStore.clear(activity)
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            val request = chain.request(); val url = request.url
            val timeline = url.encodedPath == "/api/me/message-timeline"
            val before = url.queryParameter("before")?.toInt()
            val end = before ?: newest
            if (timeline && url.queryParameter("sync") != null) { syncStarted.countDown(); heldSync?.await(3, TimeUnit.SECONDS) }
            val body = if (!timeline) "{}" else if (url.queryParameter("sync") != null) page(0, 0) else page((end - 50).coerceAtLeast(0), end)
            if (timeline) requests.add(url.toString())
            Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(if (timeline && fail) 503 else 200)
                .message("fixture").body(body.toResponseBody()).build()
        }.build()
        val show: (String, Boolean) -> Unit = { _, _ -> binding.chatPage.visibility = View.VISIBLE; binding.conversationPage.visibility = View.GONE }
        val composer = GroupAiComposer(activity, binding, { null }, "https://fixture.invalid", { "reentry-fixture" }, {}, {}, {})
        group = MainGroupChatActions(activity, binding, client, "https://fixture.invalid", {}, show,
            { _, _ -> }, {}, { _, _, _ -> }, { "reentry-fixture" }, {}, { error("unused focus") }, {}, composer)
        friend = MainFriendChatActions(activity, binding, client, "https://fixture.invalid", {}, show,
            { _, _ -> }, {}, { _, _, _ -> }, { "reentry-fixture" }, {}, {}, {})
        binding.root.measure(View.MeasureSpec.makeMeasureSpec(720, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(1280, View.MeasureSpec.EXACTLY))
        binding.root.layout(0, 0, 720, 1280)
    }
    @After fun cleanup() { heldSync?.countDown(); group.closeGroupChat(); friend.closeFriendChat(); SocialChatSnapshotStore.clear(activity); lifecycle.pause().stop().destroy() }

    @Test fun normalGroupReentryFetchesLatestAfterHistoryEvictsNewMessages() {
        group.openGroup(room, false); pump { rows().size == 50 }; repeat(3) { older() }
        assertTrue(rows().size <= 150); assertNotEquals("0239", rows().last().id)
        group.closeGroupChat(); newest = 260; group.openGroup(room, false)
        pump { rows().lastOrNull()?.id == "0259" }
        assertFalse(requests.last().contains("sync=")); assertEquals(50, rows().size)
        assertNotNull(button("加载更早消息"))
    }

    @Test fun explicitGroupRestoreRebindsNavigationEvenWithEmptySync() {
        group.openGroup(room, false); pump { rows().size == 50 }; repeat(3) { older() }
        val original = rows().map { it.id }; group.closeGroupChat()
        val count = requests.size; group.openGroup(room, false, restorePosition = true)
        pump { requests.size > count }; assertTrue(requests.last().contains("sync="))
        assertEquals(original, rows().map { it.id })
        assertNotNull("empty sync must not leave navigation closed", button("加载更早消息"))
        assertNotNull(button("回到最新消息")); older()
    }

    @Test fun friendReentryAlsoFetchesCurrentLatestPage() {
        friend.openFriend(peer, false); pump { rows().size == 50 }; repeat(3) { older() }
        friend.closeFriendChat(); newest = 260; friend.openFriend(peer, false)
        pump { rows().lastOrNull()?.id == "0259" }; assertEquals(50, rows().size)
        assertNotNull(button("加载更早消息"))
    }

    @Test fun refreshLatestIsAvailableOnLatestPageAndCanRetryAfterFailure() {
        group.openGroup(room, false); pump { rows().size == 50 }
        assertNotNull("latest refresh must remain discoverable", button("刷新最新消息"))
        fail = true; val count = requests.size; button("刷新最新消息")!!.performClick()
        pump { requests.size > count && button("刷新最新消息")?.isEnabled == true }
        assertEquals("0239", rows().last().id)
        fail = false; newest = 260; button("刷新最新消息")!!.performClick()
        pump { rows().lastOrNull()?.id == "0259" }
    }

    @Test fun latestIntentWaitsForBusySyncAndSurvivesPushCoalescing() {
        group.openGroup(room, false); pump { rows().size == 50 }
        val count = requests.size; heldSync = CountDownLatch(1)
        group.handleRealtimeMessage(room.id); assertTrue(syncStarted.await(2, TimeUnit.SECONDS))
        button("刷新最新消息")!!.performClick()
        repeat(3) { group.handleRealtimeMessage(room.id) }
        newest = 260; heldSync!!.countDown()
        pump { rows().lastOrNull()?.id == "0259" }
        val after = requests.drop(count)
        assertEquals(1, after.count { !it.contains("sync=") && !it.contains("before=") })
        assertTrue(after.size <= 3)
    }
}
