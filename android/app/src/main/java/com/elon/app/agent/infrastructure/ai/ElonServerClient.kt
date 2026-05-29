// infrastructure/ai/ElonServerClient.kt
// module: infrastructure/ai | layer: infrastructure | role: elon-server-client
// summary: 一龙服务器 AI 客户端 - 把用户输入发到服务器 CLI，由服务器 AI 回答

package com.elon.app.agent.infrastructure.ai

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

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
        conversationId: String? = null
    ): String = withContext(Dispatchers.IO) {
        val token = getAuthToken()
            ?: throw IllegalStateException("未登录，请先在 elon APP 中登录")

        val url = URL("$serverUrl/api/projects/$projectId/chat")
        Log.d(TAG, "→ POST $url  message=${message.take(40)}")

        val body = JSONObject().apply {
            put("message", message)
            if (conversationId != null) put("conversation_id", conversationId)
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
     * elon APP 的 AuthManager 把 token 存在 "auth" 表的 "auth_token" key 里。
     */
    private fun getAuthToken(): String? {
        return try {
            val prefs = context.getSharedPreferences("auth", Context.MODE_PRIVATE)
            prefs.getString("auth_token", null)?.takeIf { it.isNotBlank() }
        } catch (e: Exception) {
            Log.e(TAG, "读取 token 失败", e)
            null
        }
    }
}
