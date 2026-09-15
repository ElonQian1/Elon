package com.elon.app.sharing

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.RequestBody.Companion.asRequestBody
import org.json.JSONObject
import org.json.JSONArray
import java.net.URLEncoder
import java.util.concurrent.TimeUnit

internal data class ShareTarget(val kind: String, val id: String, val name: String)
internal class ShareTransport(private val context: Context) {
    private val owner = AuthManager.userId(context)
    private val token = AuthManager.token(context)
    private val base = ServerUrlManager.getActive(context).trimEnd('/')
    private val http = OkHttpClient.Builder().callTimeout(60, TimeUnit.SECONDS).retryOnConnectionFailure(false).build()
    fun assertOwner() { check(owner != null && owner == AuthManager.userId(context) && token == AuthManager.token(context)) { "账号已变化，请重新打开分享预览" } }
    fun targets(): List<ShareTarget> = listOf("friend", "group").flatMap { kind ->
        val values = request("/api/me/${kind}s").optJSONArray("${kind}s") ?: JSONArray()
        (0 until values.length()).map { values.getJSONObject(it) }.map {
            ShareTarget(kind, it.getString("id"), if (kind == "group") it.optString("name") else it.optString("nickname").ifBlank { it.optString("account") })
        }
    }
    fun upload(draft: ShareDraft): JSONArray {
        val refs = JSONArray()
        draft.files.forEach { a ->
            assertOwner()
            val query = mapOf("conversation_id" to "share-${draft.id}", "kind" to a.kind, "file_name" to a.fileName, "display_name" to a.displayName, "mime_type" to a.mimeType,
                "image_width" to (a.imageWidth?.toString() ?: ""), "image_height" to (a.imageHeight?.toString() ?: "")).entries.joinToString("&") { "${it.key}=${enc(it.value)}" }
            val ref = execute(Request.Builder().url("$base/api/user/${enc(owner!!)}/chat-attachments?$query").post(a.file.asRequestBody(a.mimeType.toMediaType()))).getJSONObject("attachment")
            a.sourceLink?.let { ref.put("source_link", it.json()) }; refs.put(ref)
        }
        return refs
    }
    fun send(target: ShareTarget, text: String, refs: JSONArray): JSONObject = request("/api/me/${target.kind}s/${enc(target.id)}/messages", JSONObject().put("content", text).put("attachments", refs))
    private fun request(path: String, body: JSONObject? = null): JSONObject {
        val builder = Request.Builder().url(base + path)
        if (body != null) builder.post(body.toString().toRequestBody("application/json".toMediaType()))
        return execute(builder)
    }
    private fun execute(builder: Request.Builder): JSONObject {
        assertOwner()
        http.newCall(builder.header("Authorization", "Bearer $token").build()).execute().use {
            val value = runCatching { JSONObject(it.body?.string().orEmpty()) }.getOrNull()
            check(it.isSuccessful) { "请求未完成（${it.code}），请核对网络及会话权限" }
            assertOwner(); return value ?: error("服务响应异常")
        }
    }
    private fun enc(value: String) = URLEncoder.encode(value, "UTF-8")
}
