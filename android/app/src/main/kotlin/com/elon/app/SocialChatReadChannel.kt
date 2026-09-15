package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import okhttp3.Call
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

internal class SocialChatReadError(val status: Int, message: String) : Exception(message)
internal fun Throwable.socialAccessDenied() = this is SocialChatReadError && status in listOf(401, 403, 404)
internal fun socialSession(context: Context) = "${AuthManager.userId(context)}|${AuthManager.prefs(context).getString("auth_session_revision", "")}|${AuthManager.token(context)}"

/** One bounded read per channel, a trailing refresh for push bursts, and epoch/session fencing. */
internal class SocialChatReadChannel(private val context: Context, private val http: OkHttpClient, private val server: String,
    private val normalize: (String, JSONArray) -> JSONArray = { _, rows -> rows }) {
    private val main = Handler(Looper.getMainLooper())
    @Volatile private var generation = 0L
    private var call: Call? = null
    private var requestKey: String? = null
    private var requestSession: String? = null
    private var followUp: (() -> Unit)? = null

    fun cancel() {
        generation++
        call?.cancel(); call = null; requestKey = null; requestSession = null; followUp = null
    }

    fun invalidate(key: String) {
        if (requestKey == null || requestKey == key) cancel()
        val user = AuthManager.userId(context) ?: return
        SocialChatSnapshotStore.forAccount(context, server, user).remove(key)
    }

    fun read(key: String, path: String, field: String, hydrate: Boolean = false,
             cached: (JSONArray) -> Unit = {}, value: (JSONArray) -> Unit, error: (Throwable) -> Unit) {
        if (!AuthManager.isLoggedIn(context)) { cancel(); error(SocialChatReadError(401, "请重新登录后同步消息")); return }
        if (call != null && requestKey == key && requestSession == socialSession(context)) {
            followUp = { read(key, path, field, false, cached, value, error) }
            return
        }
        cancel()
        val ticket = generation
        val session = socialSession(context)
        val user = AuthManager.userId(context) ?: return
        val store = SocialChatSnapshotStore.forAccount(context, server, user)
        val next = http.newCall(AuthManager.applyAuth(context, Request.Builder().url(server + path).header("Cache-Control", "no-cache").get()).build())
        next.timeout().timeout(12, TimeUnit.SECONDS)
        call = next; requestKey = key; requestSession = session
        fun valid() = ticket == generation && session == socialSession(context)
        io.execute {
            if (hydrate) store.read(key)?.let { rows -> main.post { if (valid()) runCatching { cached(rows) } } }
            val result = runCatching {
                next.execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) throw SocialChatReadError(response.code, runCatching { JSONObject(body).optString("error") }.getOrDefault("").ifBlank { "同步失败（${response.code}）" })
                    JSONObject(body).optJSONArray(field) ?: throw IllegalStateException("消息响应格式不正确")
                }
            }
            main.post {
                if (!valid()) return@post
                call = null; requestKey = null; requestSession = null
                val trailing = followUp; followUp = null
                result.mapCatching { raw -> normalize(key, raw).also(value) }.onSuccess { rows ->
                    io.execute { synchronized(cacheFence) { store.write(key, rows, ::valid) } }
                }.onFailure { failure ->
                    if (failure.socialAccessDenied()) store.remove(key)
                    error(failure)
                }
                if (valid()) trailing?.invoke()
            }
        }
    }

    companion object {
        internal val io = Executors.newFixedThreadPool(4)
        private val cacheFence = Any()
    }
}
