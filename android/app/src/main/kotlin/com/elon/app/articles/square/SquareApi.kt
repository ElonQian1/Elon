package com.elon.app.articles.square

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URI
import java.util.concurrent.TimeUnit

internal class SquareApi(private val context: Context) {
    fun request(path: String, method: String = "GET", body: JSONObject? = null): JSONObject = send(path, method, body?.toString()?.toRequestBody("application/json".toMediaType()))
    fun send(path: String, method: String, body: RequestBody?): JSONObject {
        val builder = Request.Builder().url(secureBase(ServerUrlManager.getActive(context)) + ROOT + path)
        if (method != "GET") builder.method(method, body ?: "{}".toRequestBody("application/json".toMediaType()))
        http.newCall(AuthManager.applyAuth(context, builder).build()).execute().use { response ->
            val bytes = response.body?.byteStream()?.use { input ->
                val output = java.io.ByteArrayOutputStream(); val chunk = ByteArray(8192)
                while (true) { val n = input.read(chunk); if (n < 0) break; output.write(chunk, 0, n); check(output.size() <= 8 * 1024 * 1024) { "服务响应过大，请联系管理员" } }
                output.toByteArray()
            } ?: error("安全服务没有返回内容，请刷新记录核实")
            val value = runCatching { JSONObject(bytes.toString(Charsets.UTF_8)) }.getOrNull()
            check(response.isSuccessful) { value?.optString("error").orEmpty().ifBlank { "安全请求失败（${response.code}），请刷新发布记录核实" } }
            return value ?: error("回执不完整，请刷新发布记录核实")
        }
    }
    companion object {
        private val http = OkHttpClient.Builder().connectTimeout(15, TimeUnit.SECONDS).callTimeout(120, TimeUnit.SECONDS).followRedirects(false).followSslRedirects(false).build()
        const val ROOT = "/api/me/article-channels/binance-square"
        const val CREATOR = "https://www.binance.com/square/creator-center/home"
        fun secureBase(raw: String): String {
            val uri = URI(raw)
            check(uri.rawUserInfo == null && uri.host != null) { "服务器地址无效" }
            if (uri.host == "43.139.149.158" && uri.port in listOf(8080, 8443)) return "https://43.139.149.158:8443"
            check(uri.scheme == "https") { "当前服务器未配置受信任HTTPS，请联系管理员" }
            return URI("https", null, uri.host, uri.port, null, null, null).toString()
        }
        fun postId(value: String): String {
            val raw = value.trim()
            if (raw.matches(Regex("[0-9]{1,64}"))) return raw
            val uri = runCatching { URI(raw) }.getOrNull()
            check(uri?.scheme == "https" && uri.host == "www.binance.com" && uri.rawUserInfo == null) { "请填写币安帖子链接或数字ID" }
            return Regex("^/(?:[\\w-]+/)?square/post/([0-9]{1,64})/?$").matchEntire(uri!!.path)?.groupValues?.get(1) ?: error("帖子链接无效")
        }
    }
}
