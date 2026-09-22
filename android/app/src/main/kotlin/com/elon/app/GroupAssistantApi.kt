package com.elon.app

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit

internal class GroupAssistantApi(private val context: Context, http: OkHttpClient, private val server: String) {
    private val client = http.newBuilder().followRedirects(false).followSslRedirects(false).callTimeout(20, TimeUnit.SECONDS).build()
    suspend fun call(group: String? = null, id: String? = null, body: JSONObject? = null, delete: Boolean = false, before: Long = 0): JSONObject {
        val session = socialSession(context)
        check(AuthManager.isLoggedIn(context)) { "请先登录一龙账号" }
        fun valid(value: String) = require(Regex("[A-Za-z0-9_-]{1,160}").matches(value))
        group?.let(::valid); id?.let(::valid)
        val path = if (group == null) "/api/me/ai-assistant" else "/api/me/groups/$group/ai-assistant" + (id?.let { "/$it" } ?: "")
        val builder = Request.Builder().url(server.trimEnd('/') + path + if (before > 0) "?before=$before" else "")
        if (delete) builder.delete() else if (body != null) builder.post(body.toString().toRequestBody("application/json".toMediaType()))
        val request = AuthManager.applyAuth(context, builder).build()
        return withContext(Dispatchers.IO) {
            client.newCall(request).execute().use { response ->
                check(session == socialSession(context) && AuthManager.isLoggedIn(context)) { "一龙账号已变化" }
                check(response.isSuccessful) { if (response.code in 400..499) "关注事项不可用、分享已取消或群权限已变化" else "群服务暂不可用，请重试" }
                val stream = requireNotNull(response.body).byteStream()
                val bytes = stream.use { it.readBytesLimited(12 * 1024 * 1024) }
                check(session == socialSession(context)) { "一龙账号已变化" }
                JSONObject(bytes.toString(Charsets.UTF_8))
            }
        }
    }
    private fun java.io.InputStream.readBytesLimited(max: Int): ByteArray {
        val output = java.io.ByteArrayOutputStream()
        val buffer = ByteArray(8192)
        while (true) { val count = read(buffer); if (count < 0) break
            check(output.size() + count <= max) { "更新内容过大" }; output.write(buffer, 0, count) }
        return output.toByteArray()
    }
}
