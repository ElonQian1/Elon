package com.elon.app.sociallinks

import android.content.Context
import android.os.Handler
import android.os.Looper
import com.elon.app.AuthManager
import com.elon.app.ChatMessageNotifications
import com.elon.app.ElonApplication
import com.elon.app.GlobalWsEvent
import com.elon.app.GlobalWsManager
import com.elon.app.ServerUrlManager
import com.elon.app.articles.ArticleApi
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URLEncoder
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/**
 * Chat messages that arrive while the reader is on screen. The reader shows them as a banner
 * with a quick reply so the user never has to leave the article to answer. Only messages from
 * other members are collected; the banner never replaces system notifications.
 */
internal object SocialLinkReaderInbox : GlobalWsManager.Listener {
    data class Conversation(val kind: String, val id: String, val name: String) {
        val key get() = "$kind:$id"
    }
    data class Arrival(val conversation: Conversation, val sender: String, val preview: String, val messageId: String)
    interface Ui { fun onInbox(count: Int, latest: Arrival?) }

    private const val MAX_ARRIVALS = 50
    private val handler = Handler(Looper.getMainLooper())
    private val arrivals = ArrayDeque<Arrival>()
    private val seen = LinkedHashSet<String>()
    private var ui: Ui? = null
    private var app: Context? = null
    /** The chat the reader was opened from; replies go here when no message arrived yet. */
    var origin: Conversation? = null; private set

    fun bind(context: Context, ui: Ui) {
        val app = context.applicationContext
        if (this.app == null) (app as? ElonApplication)?.globalWs?.addListener(this)
        this.app = app
        this.ui = ui
        ChatMessageNotifications.visibleConversation()?.let { (kind, id) ->
            origin = Conversation(kind, id, ChatMessageNotifications.displayName(app, kind, id) ?: if (kind == "group") "群聊" else "好友")
        }
        ui.onInbox(arrivals.size, arrivals.lastOrNull())
    }

    fun unbind(ui: Ui) { if (this.ui === ui) this.ui = null }

    /** Reply target: the latest arrival's chat, else the chat the reader came from. */
    fun target(): Conversation? = arrivals.lastOrNull()?.conversation ?: origin

    fun clear() { arrivals.clear(); seen.clear(); ui?.onInbox(0, null) }

    override fun onGlobalWsEvent(event: GlobalWsEvent) {
        val arrival = when (event) {
            is GlobalWsEvent.GroupMessage -> Arrival(
                Conversation("group", event.groupId, event.groupName?.takeIf { it.isNotBlank() } ?: "群聊"),
                event.senderName?.takeIf { it.isNotBlank() } ?: "群成员", preview(event.content), event.messageId)
            is GlobalWsEvent.FriendMessage -> Arrival(
                Conversation("friend", event.fromUserId, event.senderName?.takeIf { it.isNotBlank() } ?: "好友"),
                event.senderName?.takeIf { it.isNotBlank() } ?: "好友", preview(event.content), event.messageId)
            else -> return
        }
        handler.post { accept(arrival, senderId(event), app?.let { AuthManager.userId(it) }) }
    }

    private fun senderId(event: GlobalWsEvent) = when (event) {
        is GlobalWsEvent.GroupMessage -> event.fromUserId
        is GlobalWsEvent.FriendMessage -> event.fromUserId
        else -> ""
    }

    internal fun accept(arrival: Arrival, senderId: String, self: String? = null) {
        val current = ui ?: return
        if (senderId.isBlank() || senderId == self) return
        if (arrival.messageId.isNotBlank() && !seen.add(arrival.messageId)) return
        while (seen.size > MAX_ARRIVALS * 4) seen.remove(seen.first())
        arrivals.addLast(arrival)
        while (arrivals.size > MAX_ARRIVALS) arrivals.removeFirst()
        current.onInbox(arrivals.size, arrival)
    }

    private fun preview(content: String): String {
        ArticleApi.reference(content)?.let { return "[文章] ${it.optString("title").take(60)}" }
        return content.trim().replace(Regex("\\s+"), " ").take(80).ifBlank { "[新消息]" }
    }

    /** Sends a plain-text reply on a background thread; the result is delivered on the main thread. */
    fun reply(context: Context, conversation: Conversation, text: String, done: (Result<Unit>) -> Unit) {
        val app = context.applicationContext
        val body = text.trim()
        if (body.isEmpty()) { done(Result.failure(IllegalArgumentException("请输入回复内容"))); return }
        if (!AuthManager.isLoggedIn(app)) { done(Result.failure(IllegalStateException("请先登录"))); return }
        val base = ServerUrlManager.getActive(app).trimEnd('/')
        thread(name = "reader-quick-reply") {
            val result = runCatching {
                val path = "/api/me/${conversation.kind}s/${URLEncoder.encode(conversation.id, "UTF-8")}/messages"
                val builder = Request.Builder().url(base + path)
                    .post(JSONObject().put("content", body).toString().toRequestBody("application/json".toMediaType()))
                http.newCall(AuthManager.applyAuth(app, builder).build()).execute().use { response ->
                    val value = runCatching { JSONObject(response.body?.string().orEmpty()) }.getOrNull()
                    check(response.isSuccessful) { value?.optString("error").orEmpty().ifBlank { "发送失败（${response.code}）" } }
                }
            }
            handler.post { done(result) }
        }
    }

    private val http = OkHttpClient.Builder().callTimeout(30, TimeUnit.SECONDS).retryOnConnectionFailure(false).build()
}
