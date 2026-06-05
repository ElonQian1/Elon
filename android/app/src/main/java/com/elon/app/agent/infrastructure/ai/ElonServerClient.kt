// infrastructure/ai/ElonServerClient.kt
// module: infrastructure/ai | layer: infrastructure | role: elon-server-client
// summary: 一龙服务器 AI 客户端 - 把用户输入发到服务器 CLI，由服务器 AI 回答

package com.elon.app.agent.infrastructure.ai

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.TimeUnit

/**
 * 🖥️ 一龙服务器 AI 客户端
 *
 * 将用户的语音文本发送到 elon 服务器的 `/api/projects/{projectId}/chat` 接口，
 * 由服务器上的 CLI（Codex / Claude 等）处理后返回回复。
 *
 * 使用条件：
 * - 用户已登录 elon APP（AuthManager 有有效 token）
 * - 已在 Agent 设置中配置目标 project ID
 *
 * 请求格式（JSON）：
 *   { "message": "...", "conversation_id": "..." }
 *
 * 响应格式（JSON）：
 *   { "reply": "...", "task_id": "...", "conversation_id": "..." }
 */
class ElonServerClient(
    private val context: Context,
    private val serverUrl: String = "http://43.139.149.158:8080"
) {
    companion object {
        private const val TAG = "ElonServerClient"
        private const val TIMEOUT_MS = 60_000  // 服务器 CLI 可能较慢
    }

    /** OkHttp 客户端（SSE 长连接，120s readTimeout）*/
    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(120, TimeUnit.SECONDS)
        .build()

    /**
     * 向服务器发送用户消息，返回 AI 回复文本。
     *
     * @param projectId 目标项目 ID（在 AgentConfig 中配置）
     * @param message   用户输入文本
     * @param conversationId 会话 ID，null 则由服务器自动创建
     * @return AI 回复文本
     * @throws Exception 网络错误、未登录、项目不存在等
     */
    suspend fun chat(
        projectId: String,
        message: String,
        conversationId: String? = null,
        chatOnly: Boolean = false
    ): String = withContext(Dispatchers.IO) {
        val token = getAuthToken()
            ?: throw IllegalStateException("未登录，请先在 elon APP 中登录")

        val url = URL("$serverUrl/api/projects/$projectId/chat")
        Log.d(TAG, "→ POST $url  message=${message.take(40)} chatOnly=$chatOnly")

        val body = JSONObject().apply {
            put("message", message)
            if (conversationId != null) put("conversation_id", conversationId)
            // 仅闲聊：让服务器走轻量 casual chat，不启动 Codex 项目工作流（避免超时）
            if (chatOnly) put("chat_only", true)
        }.toString()

        val conn = (url.openConnection() as HttpURLConnection).apply {
            requestMethod = "POST"
            setRequestProperty("Content-Type", "application/json; charset=utf-8")
            setRequestProperty("Authorization", "Bearer $token")
            connectTimeout = TIMEOUT_MS
            readTimeout = TIMEOUT_MS
            doOutput = true
        }

        try {
            OutputStreamWriter(conn.outputStream, Charsets.UTF_8).use { it.write(body) }

            val code = conn.responseCode
            if (code != 200) {
                val err = conn.errorStream?.bufferedReader()?.readText() ?: "HTTP $code"
                Log.e(TAG, "服务器错误: $err")
                throw RuntimeException("服务器返回 $code：$err")
            }

            val resp = conn.inputStream.bufferedReader().readText()
            Log.d(TAG, "← $resp")
            val json = JSONObject(resp)
            json.optString("reply").ifBlank { "（服务器未返回回复）" }
        } finally {
            conn.disconnect()
        }
    }

    /**
     * 从 elon APP 的 SharedPreferences 读取登录 token。
     *
     * **修复**：旧版本读的是 SharedPreferences("auth")，但 [com.elon.app.AuthManager]
     * 实际把 token 存在 SharedPreferences("elon") → "auth_token"。
     * 旧代码因此永远读不到 token，agent 的服务器 CLI 兜底彻底失效。
     *
     * 现统一通过 [com.elon.app.agent.infrastructure.auth.MainAppBridge] 读取。
     */
    private fun getAuthToken(): String? {
        return try {
            com.elon.app.agent.infrastructure.auth.MainAppBridge.authToken(context)
        } catch (e: Exception) {
            Log.e(TAG, "读取 token 失败", e)
            null
        }
    }

    /**
     * SSE 流式聊天：服务器实时推送每一步进度，函数返回最终回复文本。
     *
     * @param onEvent(type, message) 在 IO 线程回调。
     *   type = "progress" | "done" | "error" | 其他 WsMessage type
     */
    suspend fun chatStream(
        projectId: String,
        message: String,
        conversationId: String? = null,
        onEvent: (type: String, message: String) -> Unit = { _, _ -> }
    ): String = withContext(Dispatchers.IO) {
        val token = getAuthToken()
            ?: throw IllegalStateException("未登录，请先在 elon APP 中登录")

        val body = JSONObject().apply {
            put("message", message)
            if (conversationId != null) put("conversation_id", conversationId)
        }.toString().toRequestBody("application/json; charset=utf-8".toMediaType())

        val request = Request.Builder()
            .url("$serverUrl/api/projects/$projectId/chat/stream")
            .post(body)
            .addHeader("Authorization", "Bearer $token")
            .addHeader("Accept", "text/event-stream")
            .build()

        Log.d(TAG, "→ SSE POST .../chat/stream  message=${message.take(40)}")
        var finalReply = ""

        httpClient.newCall(request).execute().use { response ->
            if (!response.isSuccessful) {
                val err = response.body?.string() ?: "HTTP ${response.code}"
                throw RuntimeException("服务器返回 ${response.code}：$err")
            }
            val source = response.body?.source()
                ?: throw RuntimeException("响应体为空")
            while (!source.exhausted()) {
                val line = source.readUtf8Line() ?: break
                if (!line.startsWith("data: ")) continue
                val data = line.removePrefix("data: ").trim()
                if (data.isEmpty()) continue
                runCatching {
                    val json = JSONObject(data)
                    val type = json.optString("type")
                    val msg = json.optString("message")
                    onEvent(type, msg)
                    if (type == "done") finalReply = msg
                }
            }
        }
        Log.d(TAG, "← SSE 完成，reply=${finalReply.take(60)}")
        finalReply.ifBlank { "（无回复）" }
    }
}
