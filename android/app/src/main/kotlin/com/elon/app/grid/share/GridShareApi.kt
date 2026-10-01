package com.elon.app.grid.share

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.socialSession
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.security.MessageDigest
import java.util.concurrent.TimeUnit

internal class GridShareApi(private val context: Context, http: OkHttpClient, private val server: String) {
    private val client = http.newBuilder().callTimeout(25, TimeUnit.SECONDS).followRedirects(false).followSslRedirects(false).build()
    fun request(group: String, id: String = "", body: JSONObject? = null, revoke: Boolean = false, session: String = socialSession(context)): JSONObject {
        check(AuthManager.isLoggedIn(context) && session == socialSession(context)) { "登录已变化，请重新打开分享" }
        require(listOf(group).plus(if (id.isBlank()) emptyList() else listOf(id)).all { it.matches(Regex("[A-Za-z0-9_-]{1,160}")) })
        val url = "${server.trimEnd('/')}/api/me/groups/$group/ai-snapshots" + if (id.isBlank()) "" else "/$id"
        val builder = Request.Builder().url(url).header("Cache-Control", "no-store")
        AuthManager.applyAuth(context, builder)
        if (body != null) builder.post(body.toString().toRequestBody("application/json; charset=utf-8".toMediaType()))
        if (revoke) builder.delete()
        return client.newCall(builder.build()).execute().use { response ->
            check(response.isSuccessful) { when (response.code) {
                401 -> "登录已失效，请重新登录一龙"; 403 -> "你已不在此群或无权更新"; 404, 410 -> "分享已撤回或不存在"
                409 -> "分享已有更新，请打开最新版本"; 400 -> "快照过期或数据不完整，请重新读取"
                else -> "请求未确认，请重试；同一快照不会重复发送"
            } }
            val stream = response.body?.byteStream() ?: error("响应为空")
            val bytes = stream.readBytesBounded(32 * 1024)
            check(session == socialSession(context)) { "账号已变化，请重新打开" }
            JSONObject(bytes.toString(Charsets.UTF_8))
        }
    }
    fun publish(group: String, grid: JSONObject, session: String): JSONObject {
        val document = GridShareModel.document(grid)
        // Stable across retries and app restarts; no local provider identifier participates.
        val key = MessageDigest.getInstance("SHA-256").digest(document.toString().toByteArray())
            .joinToString("") { "%02x".format(it) }
        return request(group, body = JSONObject().put("idempotency_key", key).put("document", document), session = session)
    }
    private fun java.io.InputStream.readBytesBounded(limit: Int): ByteArray {
        val output = java.io.ByteArrayOutputStream(); val buffer = ByteArray(4096)
        while (true) { val size = read(buffer); if (size < 0) break; check(output.size() + size <= limit) { "分享数据过大" }; output.write(buffer, 0, size) }
        return output.toByteArray()
    }
}
