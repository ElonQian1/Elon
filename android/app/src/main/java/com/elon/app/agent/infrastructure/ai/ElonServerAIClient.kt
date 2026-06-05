// infrastructure/ai/ElonServerAIClient.kt
// module: infrastructure/ai | layer: infrastructure | role: elon-server-adapter
// summary: 悬浮球服务器 AI 适配器：默认服务器 CLI（Codex），兜底服务器 LLM（混元）

package com.elon.app.agent.infrastructure.ai

import android.content.Context
import android.util.Log
import com.elon.app.agent.application.AIClient
import com.elon.app.agent.application.Message
import com.elon.app.agent.infrastructure.auth.MainAppBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * 悬浮球服务器 AI 适配器。
 *
 * 路径优先级：
 *   1. 服务器 CLI（Codex/Claude via /api/projects/{balloon_project_id}/chat）
 *      - 气球项目 workspace 有 AGENTS.md，Codex 按格式生成手机自动化 JSON 脚本
 *      - 也能回答闲聊（CLI 内置 intent gate：非开发任务 → casual chat 秒回）
 *   2. 服务器 LLM（混元 via /api/llm/chat）[兜底]
 *      - CLI 失败 / 超时 / 未配置项目时使用
 *      - 记忆注入和会话历史在这层保障
 *
 * 初始化时调用 /api/agent-balloon/ensure 在服务器创建"手机控制"专属项目空间
 * 并写入 AGENTS.md，后续请求带 conversationId 保持会话连续。
 */
class ElonServerAIClient(
    context: Context,
    @Suppress("UNUSED_PARAMETER") legacyProjectId: String
) : AIClient {

    private val ctx = context.applicationContext
    private val server = ElonServerClient(ctx, MainAppBridge.serverUrl(ctx))

    @Volatile private var balloonProjectId: String? = null
    @Volatile private var cliConversationId: String? = null
    @Volatile private var lmConversationId: String? = null
    private val ensureDone = java.util.concurrent.atomic.AtomicBoolean(false)

    private suspend fun ensureBalloonProject(): String? {
        if (ensureDone.get()) return balloonProjectId
        if (!ensureDone.compareAndSet(false, true)) return balloonProjectId

        return withContext(Dispatchers.IO) {
            try {
                val token = MainAppBridge.authToken(ctx) ?: run { ensureDone.set(false); return@withContext null }
                val url = java.net.URL("${MainAppBridge.serverUrl(ctx)}/api/agent-balloon/ensure")
                val conn = (url.openConnection() as java.net.HttpURLConnection).apply {
                    requestMethod = "POST"
                    setRequestProperty("Authorization", "Bearer $token")
                    setRequestProperty("Content-Length", "0")
                    connectTimeout = 10_000; readTimeout = 10_000
                }
                try {
                    if (conn.responseCode == 200) {
                        val pid = org.json.JSONObject(conn.inputStream.bufferedReader().readText()).optString("project_id")
                        if (pid.isNotBlank()) { balloonProjectId = pid; Log.i(TAG, "balloon ensured: $pid"); return@withContext pid }
                    }
                } finally { conn.disconnect() }
                ensureDone.set(false); null
            } catch (e: Exception) {
                Log.w(TAG, "ensure balloon failed (non-fatal): ${e.message}")
                ensureDone.set(false); null
            }
        }
    }

    override suspend fun chat(messages: List<Message>): String {
        val projectId = ensureBalloonProject()

        // 1. 优先走服务器 CLI（Codex，AGENTS.md 告知手机脚本格式）
        if (projectId != null) {
            try {
                val userMsg = messages.lastOrNull { it.role == "user" }?.content
                    ?: messages.joinToString("\n") { "${it.role}: ${it.content}" }
                Log.d(TAG, "-> CLI project=$projectId  msg=${userMsg.take(40)}")
                val reply = server.chat(
                    projectId = projectId,
                    message = userMsg,
                    conversationId = cliConversationId,
                    chatOnly = false
                )
                if (reply.isNotBlank() && reply != "（服务器未返回回复）") return reply
                Log.w(TAG, "CLI 返回空，降级到 LLM")
            } catch (e: Exception) {
                Log.w(TAG, "CLI 失败（${e.message}），降级到 LLM")
            }
        }

        // 2. 兜底：服务器 LLM（混元，带记忆注入）
        Log.d(TAG, "-> LLM msgs=${messages.size}  conv=${lmConversationId ?: "<new>"}")
        val (reply, newConvId) = server.lmChat(messages, conversationId = lmConversationId)
        if (newConvId != null) lmConversationId = newConvId
        return reply
    }

    companion object { private const val TAG = "ElonServerAIClient" }
}
