package com.elon.app.articles

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit

internal class ArticleApi(private val context: Context) {
    private val base = ServerUrlManager.getActive(context).trimEnd('/')
    fun request(path: String, method: String = "GET", body: JSONObject? = null): JSONObject {
        val builder = Request.Builder().url(base + path)
        if (method != "GET") builder.method(method, (body ?: JSONObject()).toString().toRequestBody("application/json".toMediaType()))
        http.newCall(AuthManager.applyAuth(context, builder).build()).execute().use { response ->
            val value = runCatching { JSONObject(response.body?.string().orEmpty()) }.getOrNull()
            check(response.isSuccessful) { value?.optString("error").orEmpty().ifBlank { "请求失败（${response.code}），请重试" } }
            return value ?: error("服务响应格式异常，请重试")
        }
    }
    fun read(id: String, revision: Long, compact: Boolean = false) = request("/api/me/articles/$id/revisions/$revision?compact=$compact")
    companion object {
        private val http = OkHttpClient.Builder().callTimeout(45, TimeUnit.SECONDS).build()
        const val PREFIX = "【一龙文章】\n"
        fun reference(content: String): JSONObject? = runCatching {
            if (!content.startsWith(PREFIX)) return null
            JSONObject(content.removePrefix(PREFIX)).takeIf {
                val revision = it.opt("revision")
                it.optInt("schema") == 1 && it.optString("article_id").matches(Regex("article_[\\w-]+")) && revision is Number && revision.toLong() > 0 && revision.toDouble() == revision.toLong().toDouble() && it.opt("title") is String && it.opt("summary") is String
            }
        }.getOrNull()
    }
}
