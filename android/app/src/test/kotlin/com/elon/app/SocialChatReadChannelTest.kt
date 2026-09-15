package com.elon.app

import android.app.Application
import android.os.Looper
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], manifest = Config.NONE, application = Application::class)
class SocialChatReadChannelTest {
    private fun signIn(id: String) {
        AuthManager.prefs(RuntimeEnvironment.getApplication()).edit().putString("auth_user_id", id)
            .putString("auth_token", "synthetic-$id").putString("auth_session_revision", id).putLong("auth_expires_at", 0).commit()
    }
    private fun pump(until: () -> Boolean) {
        repeat(150) { shadowOf(Looper.getMainLooper()).idle(); if (until()) return; Thread.sleep(10) }
        assertTrue("read channel did not settle", until())
    }

    @Test fun cachedRowsAppearWhileNetworkWaitsAndCanceledResponseCannotReplaceNewChat() {
        val context = RuntimeEnvironment.getApplication()
        signIn("a")
        val gate = CountDownLatch(1)
        val started = CountDownLatch(1)
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            if (chain.request().url.encodedPath == "/old") { started.countDown(); gate.await(2, TimeUnit.SECONDS) }
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1).code(200).message("ok")
                .body("{\"messages\":[{\"id\":\"${chain.request().url.encodedPath}\"}]}".toResponseBody()).build()
        }.build()
        val store = SocialChatSnapshotStore.forAccount(context, "https://test.invalid", "a")
        store.write("old", JSONArray("[{\"id\":\"cached\"}]"))
        val channel = SocialChatReadChannel(context, client, "https://test.invalid")
        val shown = mutableListOf<String>()
        try {
            channel.read("old", "/old", "messages", true, { shown.add(it.getJSONObject(0).getString("id")) }, { shown.add("stale") }, { })
            assertTrue(started.await(2, TimeUnit.SECONDS)); pump { shown.contains("cached") }
            channel.cancel()
            channel.read("new", "/new", "messages", value = { shown.add(it.getJSONObject(0).getString("id")) }, error = { throw it })
            gate.countDown(); pump { shown.contains("/new") }
            assertFalse(shown.contains("stale"))
        } finally { gate.countDown(); channel.cancel(); SocialChatSnapshotStore.clear(context) }
    }

    @Test fun pushBurstCoalescesAndAccountSwitchDropsLateCallbacks() {
        val context = RuntimeEnvironment.getApplication(); signIn("a")
        val gate = CountDownLatch(1); val calls = AtomicInteger()
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            calls.incrementAndGet(); gate.await(2, TimeUnit.SECONDS)
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1).code(200).message("ok").body("{\"messages\":[]}".toResponseBody()).build()
        }.build()
        val channel = SocialChatReadChannel(context, client, "https://test.invalid")
        var callbacks = 0
        try {
            repeat(10) { channel.read("g", "/g", "messages", value = { callbacks++ }, error = { }) }
            pump { calls.get() == 1 }; assertEquals(1, calls.get())
            signIn("b"); gate.countDown(); Thread.sleep(80); shadowOf(Looper.getMainLooper()).idle()
            assertEquals(0, callbacks)
            assertNull(SocialChatSnapshotStore.forAccount(context, "https://test.invalid", "b").read("g"))
            channel.read("g", "/g", "messages", value = { callbacks++ }, error = { throw it })
            pump { callbacks == 1 }
            assertEquals(2, calls.get())
        } finally { gate.countDown(); channel.cancel(); SocialChatSnapshotStore.clear(context) }
    }

    @Test fun permissionDenialRemovesSnapshotAndMalformedRowsDoNotCrashUi() {
        val context = RuntimeEnvironment.getApplication(); signIn("a")
        val store = SocialChatSnapshotStore.forAccount(context, "https://test.invalid", "a")
        store.write("g", JSONArray("[{\"id\":\"old\"}]"))
        var status = 403
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1).code(status).message("test")
                .body(if (status == 403) "{}".toResponseBody() else "{\"messages\":[1]}".toResponseBody()).build()
        }.build()
        val channel = SocialChatReadChannel(context, client, "https://test.invalid")
        var error: Throwable? = null
        try {
            channel.read("g", "/g", "messages", value = { }, error = { error = it })
            pump { error != null }; assertTrue(error!!.socialAccessDenied()); assertNull(store.read("g"))
            status = 200; error = null
            channel.read("g", "/g", "messages", value = { it.getJSONObject(0) }, error = { error = it })
            pump { error != null }; assertNull(store.read("g"))
        } finally { channel.cancel(); SocialChatSnapshotStore.clear(context) }
    }
}
