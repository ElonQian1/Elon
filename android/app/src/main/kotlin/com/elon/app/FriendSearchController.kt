package com.elon.app

import android.content.Context
import android.os.Handler
import java.io.IOException
import okhttp3.Call
import okhttp3.Callback
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import org.json.JSONObject

internal data class FriendSearchState(
    val query: String = "",
    val loading: Boolean = false,
    val users: List<JSONObject> = emptyList(),
    val message: String = ""
)

/** A query owns its response even when cancellation races with completion. */
internal class FriendSearchController(
    private val schedule: (Runnable, Long) -> Unit,
    private val unschedule: (Runnable) -> Unit,
    private val lookup: (String, (Result<JSONObject>) -> Unit) -> (() -> Unit),
    private val publish: (FriendSearchState) -> Unit
) {
    var state = FriendSearchState()
        private set
    private var revision = 0L
    private var pending: Runnable? = null
    private var cancelLookup: (() -> Unit)? = null
    private var closed = false

    fun update(input: String, immediate: Boolean = false) {
        if (closed) return
        val requestRevision = ++revision
        cancelPending()
        val query = input.trim()
        if (query.length < 2) {
            emit(FriendSearchState(query, message = if (query.isEmpty()) "" else "请输入完整手机号、账号或至少两个字的昵称"))
            return
        }
        emit(FriendSearchState(query, loading = true, message = "正在搜索..."))
        val task = Runnable {
            if (closed || revision != requestRevision) return@Runnable
            pending = null
            cancelLookup = lookup(query) { result ->
                if (!closed && revision == requestRevision) {
                    emit(result.fold(
                        onSuccess = { decode(query, it) },
                        onFailure = { FriendSearchState(query, message = it.message ?: "搜索失败，请重试") }
                    ))
                }
            }
        }
        if (immediate) task.run() else {
            pending = task
            schedule(task, 300L)
        }
    }

    fun markAdded(userId: String) {
        val user = state.users.find { it.optString("id") == userId } ?: return
        user.put("already_friend", true)
        emit(state)
    }

    fun close() {
        closed = true
        ++revision
        cancelPending()
    }

    private fun cancelPending() {
        pending?.let(unschedule)
        pending = null
        cancelLookup?.invoke()
        cancelLookup = null
    }

    private fun emit(value: FriendSearchState) {
        state = value
        publish(value)
    }

    private fun decode(query: String, data: JSONObject): FriendSearchState {
        val candidates = data.optJSONArray("results")
        if (candidates != null) {
            val users = List(candidates.length()) { candidates.optJSONObject(it) }
            if (users.any { it == null || it.isNull("id") || it.optString("id").isBlank() }) {
                return FriendSearchState(query, message = "搜索结果异常，请重试")
            }
            val validUsers = users.filterNotNull().distinctBy { it.optString("id") }
            val message = when {
                validUsers.isEmpty() -> "未找到用户，请核对完整手机号或账号"
                data.optBoolean("has_more") -> "同名用户较多，仅显示前20位，请用完整手机号或账号精确查找"
                validUsers.size > 1 -> "找到多个同名用户，请核对头像和账号信息后添加"
                validUsers.single().optBoolean("is_self") -> "这是你自己的账号，不能添加自己"
                else -> ""
            }
            return FriendSearchState(query, users = validUsers, message = message)
        }
        // Older servers return one user; keep that deployment order compatible.
        if (!data.optBoolean("found")) {
            return FriendSearchState(query, message = "未找到用户，请核对完整手机号或账号")
        }
        val user = data.optJSONObject("user")
        if (user == null || user.isNull("id") || user.optString("id").isBlank()) {
            return FriendSearchState(query, message = "搜索结果异常，请重试")
        }
        val isSelf = data.optBoolean("is_self")
        user.put("is_self", isSelf).put("already_friend", data.optBoolean("already_friend"))
        return FriendSearchState(query, users = listOf(user), message = if (isSelf) "这是你自己的账号，不能添加自己" else "")
    }
}

internal fun friendSearchLookup(
    context: Context,
    http: OkHttpClient,
    serverUrl: () -> String,
    handler: Handler
): (String, (Result<JSONObject>) -> Unit) -> (() -> Unit) = { query, complete ->
    val url = (serverUrl().trimEnd('/') + "/api/me/friends/search").toHttpUrl().newBuilder()
        .addQueryParameter("search_type", "auto")
        .addQueryParameter("include_candidates", "true")
        .addQueryParameter("query", query)
        .build()
    val call = http.newCall(AuthManager.applyAuth(context, Request.Builder().url(url).get()).build())
    call.enqueue(object : Callback {
        override fun onFailure(call: Call, error: IOException) {
            handler.post { complete(Result.failure(IOException("搜索失败，请检查网络后重试"))) }
        }

        override fun onResponse(call: Call, response: Response) {
            val result = runCatching {
                response.use {
                    val data = JSONObject(it.body?.string().orEmpty())
                    if (!it.isSuccessful) error(data.optString("error").ifBlank { "搜索失败，请重试" })
                    data
                }
            }
            handler.post { complete(result) }
        }
    })
    val cancel: () -> Unit = { call.cancel() }
    cancel
}
