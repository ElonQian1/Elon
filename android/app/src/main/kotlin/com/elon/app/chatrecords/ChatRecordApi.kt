package com.elon.app.chatrecords

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.MediaType.Companion.toMediaType
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

internal class ChatRecordApi(private val context: Context) {
    private val owner = AuthManager.userId(context)
    private val token = AuthManager.token(context)
    private val base = ServerUrlManager.getActive(context).trimEnd('/')
    private val client = OkHttpClient.Builder().callTimeout(90, TimeUnit.SECONDS).retryOnConnectionFailure(false).build()
    fun assertOwner() { check(!owner.isNullOrBlank() && owner == AuthManager.userId(context) && token == AuthManager.token(context)) { "账号已变化，请重新打开聊天记录" } }
    private fun path(group: String, suffix: String = ""): String {
        require(group.matches(Regex("[A-Za-z0-9_-]{1,128}")))
        return "$base/api/me/groups/$group/chat-records$suffix"
    }
    private fun id(value: String): String = value.also { require(it.matches(Regex("[A-Za-z0-9_-]{1,128}"))) }
    private fun execute(builder: Request.Builder): ByteArray {
        assertOwner()
        client.newCall(builder.header("Authorization", "Bearer $token").build()).execute().use { response ->
            check(response.isSuccessful) { when (response.code) {
                401 -> "请先登录一龙"; 403, 404 -> "记录已撤回，或你已不在此群聊中"; 413 -> "记录或附件超过存储限制"; else -> "聊天记录请求失败（${response.code}），请重试"
            } }
            val body = response.body ?: error("服务响应为空")
            require(body.contentLength() <= 12 * 1024 * 1024) { "附件过大" }
            val bytes = body.byteStream().use { input ->
                val out = java.io.ByteArrayOutputStream(); val buffer = ByteArray(16384)
                while (true) { val n = input.read(buffer); if (n < 0) break
                    require(out.size() + n <= 12 * 1024 * 1024) { "附件过大" }; out.write(buffer, 0, n) }
                out.toByteArray()
            }
            assertOwner(); return bytes
        }
    }
    fun upload(group: String, file: File): String = JSONObject(String(execute(Request.Builder().url(path(group, "/assets"))
        .post(file.asRequestBody("application/octet-stream".toMediaType()))), Charsets.UTF_8)).getString("asset_id")
    fun publish(group: String, operation: String, document: ChatRecordDocument): JSONObject = JSONObject(String(execute(
        Request.Builder().url(path(group)).post(JSONObject().put("operation", operation).put("document", document.json())
            .toString().toRequestBody("application/json".toMediaType()))), Charsets.UTF_8))
    fun read(group: String, record: String): JSONObject = JSONObject(String(execute(Request.Builder().url(path(group, "/${id(record)}"))), Charsets.UTF_8))
    fun asset(group: String, record: String, asset: String): ByteArray = execute(Request.Builder().url(path(group, "/${id(record)}/assets/${id(asset)}")))
    fun revoke(group: String, record: String) { execute(Request.Builder().url(path(group, "/${id(record)}")).delete()) }
}
