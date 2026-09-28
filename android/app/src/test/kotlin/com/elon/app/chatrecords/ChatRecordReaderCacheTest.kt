package com.elon.app.chatrecords

import android.app.Application
import android.os.Looper
import androidx.lifecycle.ViewModelStore
import com.elon.app.AuthManager
import java.net.InetAddress
import java.net.ServerSocket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordReaderCacheTest {
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val stores = mutableListOf<ViewModelStore>()
    private val pool = Executors.newCachedThreadPool()
    private val entered = CountDownLatch(1)
    private val release = CountDownLatch(1)
    private lateinit var server: ServerSocket
    @Volatile private var blocked = false
    @Volatile private var status = 304
    @Before fun setup() {
        val doc = ChatRecordDocument("Fixture", "Fixture", listOf(RecordRow("m1", null, "Fixture", "", "text", "Fixture body")))
        val bytes = JSONObject().put("owner_id", "fixture-owner").put("document", doc.json()).toString().toByteArray()
        server = ServerSocket(0, 16, InetAddress.getByName("127.0.0.1"))
        pool.execute {
            while (!server.isClosed) {
                val socket = runCatching { server.accept() }.getOrNull() ?: break
                pool.execute {
                    socket.use {
                        it.soTimeout = 3000
                        val input = it.getInputStream().bufferedReader()
                        while (!input.readLine().isNullOrEmpty()) { /* Consume only the synthetic request headers. */ }
                        if (blocked) { entered.countDown(); check(release.await(5, TimeUnit.SECONDS)) }
                        val code = if (blocked) status else 200
                        val body = if (code == 200) bytes else byteArrayOf()
                        val header = "HTTP/1.1 $code Fixture\r\nETag: fixture-v1\r\nContent-Length: ${body.size}\r\nConnection: close\r\n\r\n"
                        runCatching { it.getOutputStream().write(header.toByteArray() + body); it.getOutputStream().flush() }
                    }
                }
            }
        }
        AuthManager.prefs(app).edit().clear().putString("auth_user_id", "fixture-owner")
            .putString("auth_token", "synthetic-session").commit()
        app.getSharedPreferences("agent_config", 0).edit().putString("fallback_server_url", "http://127.0.0.1:${server.localPort}").commit()
        app.getSharedPreferences("server_url_mgr", 0).edit().putBoolean("use_fallback", true)
            .putLong("fallback_since_ms", System.currentTimeMillis()).commit()
        val api = ChatRecordApi(app)
        try { api.read("fixture-group", "fixture-record") } finally { api.close() }
        blocked = true
    }
    @After fun teardown() {
        stores.forEach { it.clear() }; release.countDown(); server.close(); pool.shutdownNow()
        shadowOf(Looper.getMainLooper()).idle()
        ChatRecordCache.clearAll(java.io.File(app.cacheDir, "chat_record_cache_v1"))
    }
    private fun open(): ChatRecordReaderModel = ChatRecordReaderModel(app).also {
        stores.add(ViewModelStore().apply { put("reader", it) }); it.start("fixture-group", "fixture-record", "")
    }
    private fun until(condition: () -> Boolean) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (System.nanoTime() < deadline) { shadowOf(Looper.getMainLooper()).idle(); if (condition()) return; Thread.sleep(10) }
        fail("Reader state did not converge")
    }
    @Test fun reopenedModelDisplaysLocalDocumentWhileValidationIsStillBlocked() {
        val model = open(); assertTrue(entered.await(2, TimeUnit.SECONDS))
        until { model.document != null }; assertTrue(model.loading)
        release.countDown(); until { !model.loading }; assertNotNull(model.document)
    }
    @Test fun deniedBackgroundValidationRemovesAlreadyDisplayedDocument() {
        status = 403
        val model = open(); assertTrue(entered.await(2, TimeUnit.SECONDS)); until { model.document != null }
        release.countDown(); until { !model.loading }; assertNull(model.document)
        assertNull(ChatRecordApi(app).peek("fixture-group", "fixture-record"))
    }
    @Test fun closingApiCancelsBlockedRequestWithoutWaitingForServer() {
        val api = ChatRecordApi(app)
        val request = pool.submit<Throwable?> { runCatching { api.read("fixture-group", "fixture-record") }.exceptionOrNull() }
        assertTrue(entered.await(2, TimeUnit.SECONDS)); api.close()
        assertNotNull(request.get(2, TimeUnit.SECONDS))
        assertTrue(runCatching { api.read("fixture-group", "fixture-record") }.isFailure)
    }
}
