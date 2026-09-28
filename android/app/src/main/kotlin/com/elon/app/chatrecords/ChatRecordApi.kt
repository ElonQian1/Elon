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
    private val root = File(context.cacheDir, "chat_record_cache_v1")
    private val calls = mutableSetOf<okhttp3.Call>()
    private var closed = false
    private fun cache(group: String, record: String) = ChatRecordCache(root, "$base\n$owner\n$token\n$group\n$record")
    fun cancelPending() = synchronized(calls) { calls.toList().forEach { it.cancel() } }
    fun close() = synchronized(calls) { closed = true; cancelPending() }
    private fun <T> request(client: OkHttpClient, request: Request, body: (okhttp3.Response) -> T): T {
        val call = client.newCall(request)
        synchronized(calls) { check(!closed) { "记录已关闭" }; calls.add(call) }
        try { return call.execute().use(body) } finally { synchronized(calls) { calls.remove(call) } }
    }
    fun assertOwner() { check(AuthManager.isLoggedIn(context) && owner == AuthManager.userId(context) && token == AuthManager.token(context) && base == ServerUrlManager.getActive(context).trimEnd('/')) { "登录已过期或账号已变化，请重新打开聊天记录" } }
    private fun path(group: String, suffix: String = ""): String {
        require(group.matches(Regex("[A-Za-z0-9_-]{1,128}")))
        return "$base/api/me/groups/$group/chat-records$suffix"
    }
    private fun id(value: String): String = value.also { require(it.matches(Regex("[A-Za-z0-9_-]{1,128}"))) }
    private fun cached(group: String, record: String, url: String, client: OkHttpClient): File = cache(group, record).read(url) { etag ->
        assertOwner()
        val builder = Request.Builder().url(url).header("Authorization", "Bearer $token")
        if (etag != null) builder.header("If-None-Match", etag)
        request(client, builder.build()) { response ->
            val bytes = if (response.isSuccessful) boundedBody(response) else byteArrayOf()
            assertOwner()
            ChatRecordCache.Download(response.code, response.header("ETag"), bytes)
        }
    }
    private fun boundedBody(response: okhttp3.Response): ByteArray {
        val body = response.body ?: error("服务响应为空")
        require(body.contentLength() <= 12 * 1024 * 1024) { "附件过大" }
        return body.byteStream().use { input ->
            val out = java.io.ByteArrayOutputStream(); val buffer = ByteArray(16384)
            while (true) { val n = input.read(buffer); if (n < 0) break
                require(out.size() + n <= 12 * 1024 * 1024) { "附件过大" }; out.write(buffer, 0, n) }
            out.toByteArray()
        }
    }
    private fun execute(builder: Request.Builder): ByteArray {
        assertOwner()
        return request(client, builder.header("Authorization", "Bearer $token").build()) { response ->
            check(response.isSuccessful) { when (response.code) {
                401 -> "请先登录一龙"; 403, 404 -> "记录已撤回，或你已不在此群聊中"; 413 -> "记录或附件超过存储限制"; else -> "聊天记录请求失败（${response.code}），请重试"
            } }
            val bytes = boundedBody(response)
            assertOwner(); bytes
        }
    }
    fun upload(group: String, file: File): String = JSONObject(String(execute(Request.Builder().url(path(group, "/assets"))
        .post(file.asRequestBody("application/octet-stream".toMediaType()))), Charsets.UTF_8)).getString("asset_id")
    fun publish(group: String, operation: String, document: ChatRecordDocument): JSONObject = JSONObject(String(execute(
        Request.Builder().url(path(group)).post(JSONObject().put("operation", operation).put("document", document.json())
            .toString().toRequestBody("application/json".toMediaType()))), Charsets.UTF_8))
    fun peek(group: String, record: String): JSONObject? {
        assertOwner()
        val value = cache(group, record).fresh(path(group, "/${id(record)}"))?.let { runCatching { JSONObject(it.readText()) }.getOrNull() }
        assertOwner(); return value
    }
    fun read(group: String, record: String): JSONObject = JSONObject(cached(group, record, path(group, "/${id(record)}"), documentClient).readText())
    fun assetFile(group: String, record: String, asset: String): File {
        assertOwner(); val cache = cache(group, record); val url = path(group, "/${id(record)}/assets/${id(asset)}")
        // An authorized document grants this record's immutable cached assets the same short lease.
        if (cache.fresh(path(group, "/${id(record)}")) != null) cache.fresh(url, ChatRecordCache.RETAIN_MS)?.let { assertOwner(); return it }
        return cached(group, record, url, readClient)
    }
    fun usage(group: String, record: String): Pair<Long, Long> = cache(group, record).bytes() to ChatRecordCache.totalBytes(root)
    fun clearCache(group: String, record: String, all: Boolean) {
        assertOwner(); cancelPending()
        if (all) ChatRecordCache.clearAll(root) else cache(group, record).clear()
    }
    fun revoke(group: String, record: String) { execute(Request.Builder().url(path(group, "/${id(record)}")).delete()); cancelPending(); cache(group, record).clear() }
    companion object {
        private val client = OkHttpClient.Builder().callTimeout(90, TimeUnit.SECONDS).retryOnConnectionFailure(false).build()
        private val readClient = client.newBuilder().callTimeout(25, TimeUnit.SECONDS).build()
        private val documentClient = client.newBuilder().connectTimeout(5, TimeUnit.SECONDS).callTimeout(8, TimeUnit.SECONDS).build()
    }
}
