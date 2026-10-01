package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import okhttp3.Call
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.MediaType.Companion.toMediaType
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
    private var timelineSession = ""
    private val timelines = linkedMapOf<String, MessageTimelineWindow>()
    var canMarkRead: () -> Boolean = { false }
    fun timeline(key: String): MessageTimelineWindow? = timelines[key]
    private fun window(key: String): MessageTimelineWindow {
        val session = socialSession(context)
        if (timelineSession != session) { timelines.clear(); timelineSession = session }
        val value = timelines.getOrPut(key) { MessageTimelineWindow() }
        while (timelines.size > 10) timelines.remove(timelines.keys.first { it != key })
        return value
    }
    private var followUp: (() -> Unit)? = null

    fun cancel() {
        generation++
        call?.cancel(); call = null; requestKey = null; requestSession = null; followUp = null
    }

    fun invalidate(key: String) {
        if (requestKey == null || requestKey == key) cancel()
        timelines.remove(key)
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
        val timeline = if (field == "messages" && (key.startsWith("friend:") || key.startsWith("group:"))) window(key) else null
        val direction = timeline?.direction() ?: "sync"
        val requestPath = timeline?.path(key, direction) ?: path
        val builder = Request.Builder().url(server + requestPath).header("Cache-Control", "no-cache")
        if (direction == "window") builder.post(timeline!!.windowRequest(key).toString().toRequestBody("application/json".toMediaType())) else builder.get()
        val next = http.newCall(AuthManager.applyAuth(context, builder).build())
        next.timeout().timeout(12, TimeUnit.SECONDS)
        call = next; requestKey = key; requestSession = session
        fun valid() = ticket == generation && session == socialSession(context)
        io.execute {
            if (hydrate) store.read(key)?.let { rows -> main.post { if (valid()) runCatching { cached(rows) } } }
            val result = runCatching {
                next.execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) throw SocialChatReadError(response.code, runCatching { JSONObject(body).optString("error") }.getOrDefault("").ifBlank { "同步失败（${response.code}）" })
                    JSONObject(body)
                }
            }
            main.post {
                if (!valid()) return@post
                call = null; requestKey = null; requestSession = null
                val trailing = followUp; followUp = null
                var changed = true
                val accepted = result.mapCatching { payload ->
                    changed = timeline == null || direction != "sync" || payload.optJSONArray("messages")?.length() != 0 || payload.optJSONArray("removed_ids")?.length() != 0
                    val raw = timeline?.accept(payload, direction) ?: payload.optJSONArray(field) ?: throw IllegalStateException("消息响应格式不正确")
                    normalize(key, raw).also { if (changed) value(it) }
                }.onSuccess { rows ->
                    if (changed) io.execute { synchronized(cacheFence) { store.write(key, rows, ::valid) } }
                }.onFailure { failure ->
                    if (failure.socialAccessDenied()) { store.remove(key); timelines.remove(key) }
                    if (failure is SocialChatReadError && failure.status == 400) timeline?.latest()
                    error(failure)
                }
                if (valid() && accepted.isSuccess && canMarkRead()) timeline?.readReceipt(key)?.let { receipt ->
                    val builder = AuthManager.applyAuth(context, Request.Builder().url(server + "/api/me/message-timeline/read"))
                    val readCall = http.newCall(builder.post(receipt.toString().toRequestBody("application/json".toMediaType())).build())
                    readCall.timeout().timeout(12, TimeUnit.SECONDS)
                    io.execute { if (valid()) runCatching { readCall.execute().use { response ->
                        if (response.isSuccessful) main.post { if (valid()) timeline.lastRead = receipt.getString("message_id") }
                    } } }
                }
                if (valid()) {
                    if (trailing != null) trailing.invoke()
                    else if (accepted.isSuccess && timeline?.moreChanges == true) read(key, path, field, false, cached, value, error)
                }
            }
        }
    }

    companion object {
        internal val io = Executors.newFixedThreadPool(4)
        private val cacheFence = Any()
    }
}
