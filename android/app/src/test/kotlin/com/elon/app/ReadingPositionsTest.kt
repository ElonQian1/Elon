package com.elon.app

import android.app.Application
import android.os.Looper
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.io.IOException
import java.util.concurrent.CopyOnWriteArrayList

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], manifest = Config.NONE, application = Application::class)
class ReadingPositionsTest {
    private fun pump(until: () -> Boolean) { repeat(300) { shadowOf(Looper.getMainLooper()).idle(); if (until()) return; Thread.sleep(10) }; assertTrue(until()) }
    @Test fun firstOnlineLoadEstablishesOrdinaryProgressRevisionBeforeReading() {
        val context = RuntimeEnvironment.getApplication()
        AuthManager.prefs(context).edit().putString("auth_user_id", "initial-reading-fixture").putString("auth_token", "fixture-token").putLong("auth_expires_at", 0).commit()
        context.getSharedPreferences("reading_positions_v1", 0).edit().clear().commit()
        val requests = CopyOnWriteArrayList<JSONObject>()
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            val request = chain.request()
            val body = if (request.url.encodedPath.endsWith("capabilities")) "{\"reading_bookmarks\":true}"
                else if (request.method == "GET") "{\"bookmarks\":[],\"conversation_progress\":{\"position\":{\"message_id\":\"previous\"},\"revision\":7}}"
                else { val buffer = okio.Buffer(); request.body!!.writeTo(buffer); val op = JSONObject(buffer.readUtf8()); requests.add(op)
                    JSONObject().put("progress", JSONObject().put("position", op.getJSONObject("position")).put("revision", 8)).toString() }
            Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(200).message("fixture").body(body.toResponseBody()).build()
        }.build()
        val model = ReadingPositions(context, client, "https://test.invalid", "group:g") {}
        try {
            model.load(); pump { model.supported }
            model.putPosition(JSONObject().put("message_id", "current")); model.flush(); pump { model.pending == 0 }
            assertEquals(7L, requests.single().getLong("base_revision"))
        } finally { model.close() }
    }
    @Test fun separateBookmarksAndOfflineRestartPreserveAnchorsProgressAndOperationIds() {
        val context = RuntimeEnvironment.getApplication()
        AuthManager.prefs(context).edit().putString("auth_user_id", "reading-fixture").putString("auth_token", "fixture-token").putLong("auth_expires_at", 0).commit()
        context.getSharedPreferences("reading_positions_v1", 0).edit().clear().commit()
        val requests = CopyOnWriteArrayList<JSONObject>()
        var offline = false
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            if (offline) throw IOException("fixture offline")
            val request = chain.request()
            val body = if (request.url.encodedPath.endsWith("capabilities")) "{\"reading_bookmarks\":true}"
                else if (request.method == "GET") "{\"bookmarks\":[],\"next\":null}"
                else { val buffer = okio.Buffer(); request.body!!.writeTo(buffer); val op = JSONObject(buffer.readUtf8()); requests.add(op)
                    if (op.optString("action") == "progress") JSONObject().put("progress", JSONObject().put("position", op.getJSONObject("position")).put("revision", requests.size)).toString() else "{\"revision\":1}" }
            Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(200).message("ok").body(body.toResponseBody()).build()
        }.build()
        val model = ReadingPositions(context, client, "https://test.invalid", "group:g") {}
        model.load(); pump { model.supported }
        offline = true
        val a = model.add(ChatMessage("user", "", id = "m1"), "A", "")!!
        val b = model.add(ChatMessage("user", "", id = "m2"), "B", "")!!
        model.activate(a); model.putPosition(JSONObject().put("message_id", "m10").put("fraction", .4))
        model.activate(b); model.putPosition(JSONObject().put("message_id", "m20").put("fraction", .6))
        model.flush(); pump { model.error.isNotEmpty() }; model.close()
        val restored = ReadingPositions(context, client, "https://test.invalid", "group:g") {}
        assertTrue(restored.supported)
        assertEquals("m1", restored.position(a, false)!!.getString("message_id"))
        assertEquals("m10", restored.position(a)!!.getString("message_id"))
        assertEquals("m20", restored.position(b)!!.getString("message_id"))
        offline = false; restored.flush(); pump { restored.pending == 0 }
        assertEquals(4, requests.size)
        restored.activate(""); restored.putPosition(JSONObject().put("message_id", "latest"))
        assertEquals("m10", restored.position(a)!!.getString("message_id")); restored.close()
    }
}
