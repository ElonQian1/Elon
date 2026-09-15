package com.elon.app

import android.app.Application
import android.os.Looper
import android.view.View
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
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
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], application = Application::class)
class SocialChatStatusTest {
    private lateinit var lifecycle: ActivityController<AppCompatActivity>
    private lateinit var activity: AppCompatActivity
    private lateinit var binding: ActivityMainBinding
    private lateinit var friend: MainFriendChatActions
    private val requestStarted = CountDownLatch(1)
    private val releaseRead = CountDownLatch(1)
    private val readFinished = CountDownLatch(1)
    private var statusCode = 200
    private var response = "{\"messages\":[{\"id\":\"remote\",\"content\":\"已同步的消息\",\"sender_user_id\":\"peer\",\"created_at\":\"2026-09-15T10:00:00Z\"}]}"
    private val peer = AppFriend("peer", "测试好友", "peer", null, null, null, null, null, 0)
    private lateinit var client: OkHttpClient
    private fun status(): TextView? = binding.chatListFrame.findViewWithTag("social-sync-status")
    private fun assertHidden() = assertTrue("sync overlay must not cover the active message surface", status()?.visibility != View.VISIBLE)
    private fun pump(until: () -> Boolean) {
        repeat(300) { shadowOf(Looper.getMainLooper()).idle(); if (until()) return; Thread.sleep(10) }
        assertTrue("UI read did not settle", until())
    }

    @Before fun setup() {
        lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        activity = lifecycle.get(); activity.setTheme(R.style.Theme_ElonApp); lifecycle.setup()
        binding = ActivityMainBinding.inflate(activity.layoutInflater)
        AuthManager.prefs(activity).edit().putString("auth_user_id", "status-test")
            .putString("auth_token", "synthetic-status-test").putString("auth_session_revision", "status-test")
            .putLong("auth_expires_at", 0).commit()
        SocialChatSnapshotStore.clear(activity)
        client = OkHttpClient.Builder().addInterceptor { chain ->
            if (chain.request().method == "GET") {
                requestStarted.countDown(); releaseRead.await(3, TimeUnit.SECONDS); readFinished.countDown()
            }
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1)
                .code(if (chain.request().method == "GET") statusCode else 503).message("fixture")
                .body((if (chain.request().method == "GET") response else "{\"error\":\"发送失败\"}").toResponseBody()).build()
        }.build()
        friend = MainFriendChatActions(activity, binding, client, "https://status.invalid", {}, { _, _ -> },
            { _, _ -> }, {}, { _, _, _ -> }, { "status-test" }, {}, {}, {})
    }

    @After fun cleanup() {
        releaseRead.countDown(); friend.closeFriendChat()
        client.dispatcher.executorService.shutdownNow(); SocialChatSnapshotStore.clear(activity)
        lifecycle.pause().stop().destroy()
    }

    @Test fun aiChatHandoffClearsTheCanceledFriendOverlayAndLateResponse() {
        friend.openFriend(peer, false)
        assertEquals("正在同步好友消息…", status()?.text.toString())
        assertTrue(requestStarted.await(2, TimeUnit.SECONDS))
        friend.suspendForExternalChat()
        assertHidden()
        binding.chatList.adapter = ChatAdapter(mutableListOf(ChatMessage("assistant", "AI 聊天已经有内容")))
        releaseRead.countDown(); assertTrue(readFinished.await(2, TimeUnit.SECONDS))
        Thread.sleep(50); shadowOf(Looper.getMainLooper()).idle()
        assertHidden(); assertEquals(1, binding.chatList.adapter!!.itemCount)
    }

    @Test fun initialSuccessClearsLoadingAndBackgroundRefreshDoesNotRestoreIt() {
        releaseRead.countDown(); friend.openFriend(peer, false)
        pump { friend.currentMessages().isNotEmpty() }
        assertHidden()
        friend.handleRealtimeMessage(peer.id)
        assertHidden()
        friend.stopPolling(); assertHidden()
        friend.resumeIfActive(); assertHidden()
    }

    @Test fun pendingMessageImmediatelyRemovesLoadingEvenBeforeSendAcknowledgement() {
        friend.openFriend(peer, false)
        assertTrue(requestStarted.await(2, TimeUnit.SECONDS))
        friend.trySendMessage("新消息", emptyList())
        assertEquals(1, friend.currentMessages().size)
        assertHidden()
    }

    @Test fun forwardedMessageAlsoRemovesTheEmptyListOverlay() {
        friend.openFriend(peer, false)
        friend.trySendForwardedMessage(ChatMessage("friend", "转发内容"))
        assertHidden()
    }

    @Test fun emptyFailureCanRetryAndClearingRemovesItsAction() {
        statusCode = 503; response = "{}"; releaseRead.countDown(); friend.openFriend(peer, false)
        pump { status()?.text?.contains("点击重试") == true }
        statusCode = 200; response = "{\"messages\":[]}"
        status()!!.performClick()
        pump { status()?.text.toString() == "还没有消息" }
        assertFalse(status()!!.hasOnClickListeners())
        friend.stopPolling(); assertHidden(); assertFalse(status()!!.hasOnClickListeners())
    }

    @Test fun sharedStatusCannotCoverAlreadyVisibleMessagesOrCreateAnEmptyOverlay() {
        showSocialChatStatus(binding, null)
        assertNull(status())
        binding.chatList.adapter = ChatAdapter(mutableListOf(ChatMessage("assistant", "可见消息")))
        showSocialChatStatus(binding, "正在同步好友消息…") {}
        assertHidden()
        assertFalse(status()?.hasOnClickListeners() ?: false)
    }

    @Test fun groupPauseClearsTheSameSharedOverlay() {
        val composer = GroupAiComposer(activity, binding, { null }, "https://status.invalid", { "status-test" }, {}, {}, {})
        val group = MainGroupChatActions(activity, binding, client, "https://status.invalid", {}, { _, _ -> },
            { _, _ -> }, {}, { _, _, _ -> }, { "status-test" }, {}, { error("unused focus path") }, {}, composer)
        showSocialChatStatus(binding, "正在同步群聊消息…") {}
        group.stopPolling(); assertHidden(); assertFalse(status()!!.hasOnClickListeners())
    }
}
