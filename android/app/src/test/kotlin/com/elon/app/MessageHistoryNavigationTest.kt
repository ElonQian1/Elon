package com.elon.app

import android.app.Application
import android.app.Activity
import android.os.Looper
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], manifest = Config.NONE, application = Application::class)
class MessageHistoryNavigationTest {
    private fun pump(details: () -> String = { "" }, until: () -> Boolean) {
        repeat(250) { shadowOf(Looper.getMainLooper()).idle(); if (until()) return; Thread.sleep(10) }
        assertTrue("history request did not settle ${details()}", until())
    }
    private fun page(start: Int, end: Int, more: Boolean) = JSONObject().put("schema", "elon.message_timeline.v1")
        .put("messages", JSONArray((start until end).map { n -> JSONObject().put("id", n.toString().padStart(4, '0'))
            .put("created_at", "2026-10-01").put("content", "message $n").put("timeline_cursor", n.toString()) }))
        .put("removed_ids", JSONArray()).put("has_more", more).put("sync", "checkpoint").toString()

    @Test fun realRecyclerGestureLoadsOlderAndPreservesAnchorWithBoundedWindow() {
        val controller = Robolectric.buildActivity(Activity::class.java).setup()
        val context = controller.get()
        AuthManager.prefs(context).edit().putString("auth_user_id", "edge-fixture").putString("auth_token", "synthetic")
            .putString("auth_session_revision", "edge").putLong("auth_expires_at", 0).commit()
        val calls = AtomicInteger(); val olderCalls = AtomicInteger(); var failOlder = false
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            calls.incrementAndGet()
            val before = chain.request().url.queryParameter("before")?.toInt()
            if (before != null) olderCalls.incrementAndGet()
            val denied = before != null && failOlder
            val end = before ?: 200
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1).code(if (denied) 503 else 200).message("fixture")
                .body((if (denied) "{}" else page((end - 50).coerceAtLeast(0), end, end > 50)).toResponseBody()).build()
        }.build()
        val reader = SocialChatReadChannel(context, client, "https://test.invalid")
        val list = RecyclerView(context); val parent = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL; addView(list) }
        context.setContentView(parent)
        val manager = LinearLayoutManager(context); list.layoutManager = manager; list.itemAnimator = null
        val rows = mutableListOf<ChatMessage>()
        val adapter = object : RecyclerView.Adapter<RecyclerView.ViewHolder>() {
            override fun onCreateViewHolder(p: ViewGroup, type: Int): RecyclerView.ViewHolder = object : RecyclerView.ViewHolder(TextView(context).apply { layoutParams = RecyclerView.LayoutParams(-1, 60) }) {}
            override fun getItemCount() = rows.size
            override fun onBindViewHolder(holder: RecyclerView.ViewHolder, position: Int) { (holder.itemView as TextView).text = rows[position].content }
        }
        list.adapter = adapter
        fun layout() { list.measure(View.MeasureSpec.makeMeasureSpec(390, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(360, View.MeasureSpec.EXACTLY)); list.layout(0, 0, 390, 360) }
        val navigation = MessageTimelineNavigation(list, list, reader)
        var failure: Throwable? = null
        fun read() { reader.read("group:g", "/unused", "messages", value = { data ->
            rows.clear(); repeat(data.length()) { rows.add(ChatMessage("assistant", data.getJSONObject(it).getString("content"), id = data.getJSONObject(it).getString("id"))) }
            adapter.notifyDataSetChanged(); navigation.update("group:g", rows) { read() }; layout()
        }, error = { failure = it }) }
        fun gesture(towardLatest: Boolean = false) {
            val now = android.os.SystemClock.uptimeMillis()
            val start = if (towardLatest) 330f else 20f; val end = if (towardLatest) 20f else 330f
            for ((action, y) in listOf(MotionEvent.ACTION_DOWN to start, MotionEvent.ACTION_MOVE to end, MotionEvent.ACTION_UP to end)) {
                val event = MotionEvent.obtain(now, now + if (action == MotionEvent.ACTION_DOWN) 0 else 60, action, 100f, y, 0)
                list.dispatchTouchEvent(event); event.recycle()
            }
        }
        try {
            read(); pump { rows.size == 50 }; layout()
            assertEquals(1, calls.get()) // Initial layout must not walk the entire conversation.
            manager.scrollToPositionWithOffset(0, 0); layout()
            gesture(); pump { rows.first().id == "0100" }; layout()
            assertEquals(1, olderCalls.get()); assertEquals("0150", rows[manager.findFirstVisibleItemPosition()].id)
            repeat(2) { manager.scrollToPositionWithOffset(0, 0); layout(); gesture(); val expected = 2 + it; pump { olderCalls.get() == expected && !reader.isReading("group:g") }; layout() }
            assertEquals("0000", rows.first().id); assertTrue(rows.size <= 150)
            manager.scrollToPositionWithOffset(0, 0); layout(); gesture(); shadowOf(Looper.getMainLooper()).idle()
            assertEquals(3, olderCalls.get()) // hasOlder=false at the real beginning.
            reader.timeline("group:g")!!.latest(); read(); pump { rows.size == 50 && !reader.isReading("group:g") }
            failOlder = true; manager.scrollToPositionWithOffset(0, 0); layout(); gesture(); pump { failure != null }
            assertEquals(50, rows.size)
            failOlder = false; gesture(); pump({ "olderCalls=${olderCalls.get()} reading=${reader.isReading("group:g")} first=${rows.first().id} position=${manager.findFirstVisibleItemPosition()} mode=${reader.timeline("group:g")?.nextDirection}" }) { rows.first().id == "0100" }; assertTrue(rows.size <= 150)
            manager.scrollToPosition(rows.lastIndex); layout(); gesture(towardLatest = true)
            pump { rows.size == 50 && rows.first().id == "0150" }
            assertEquals("0199", rows.last().id)
            navigation.close(); val count = calls.get(); manager.scrollToPositionWithOffset(0, 0); layout(); gesture()
            assertEquals(count, calls.get())
        } finally { navigation.close(); reader.cancel(); SocialChatSnapshotStore.clear(context); parent.removeAllViews(); controller.pause().stop().destroy() }
    }

    @Test fun historyRequestedDuringPollingIsNotReplacedByThatPollResponse() {
        val context = RuntimeEnvironment.getApplication()
        AuthManager.prefs(context).edit().putString("auth_user_id", "queue-fixture").putString("auth_token", "synthetic")
            .putString("auth_session_revision", "queue").putLong("auth_expires_at", 0).commit()
        val gate = CountDownLatch(1); val syncStarted = CountDownLatch(1); val olderCalls = AtomicInteger()
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            val url = chain.request().url
            if (url.queryParameter("sync") != null) { syncStarted.countDown(); gate.await(3, TimeUnit.SECONDS) }
            val before = url.queryParameter("before")?.toInt()
            if (before != null) olderCalls.incrementAndGet()
            val body = if (url.queryParameter("sync") != null) page(0, 0, false) else page((before ?: 200) - 50, before ?: 200, true)
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1).code(200).message("fixture").body(body.toResponseBody()).build()
        }.build()
        val reader = SocialChatReadChannel(context, client, "https://test.invalid"); var first = ""
        fun read() = reader.read("group:g", "/unused", "messages", value = { first = it.getJSONObject(0).getString("id") }, error = { throw it })
        try {
            read(); pump { first == "0150" }; read(); assertTrue(syncStarted.await(2, TimeUnit.SECONDS))
            reader.timeline("group:g")!!.older(); repeat(3) { read() }; gate.countDown()
            pump { first == "0100" }; assertEquals(1, olderCalls.get())
        } finally { gate.countDown(); reader.cancel(); SocialChatSnapshotStore.clear(context) }
    }
}
