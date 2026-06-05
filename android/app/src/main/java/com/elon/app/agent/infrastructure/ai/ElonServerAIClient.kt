// infrastructure/ai/ElonServerAIClient.kt
// module: infrastructure/ai | layer: infrastructure | role: elon-server-adapter
// summary: 悬浮球服务器 AI 适配器：WS 全双工优先，HTTP CLI 次之，HTTP LLM 兜底

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
 * ## 通信优先级
 *   1. **WebSocket 全双工**（`/ws/projects/{balloon_project_id}`）
 *      - 长连接复用，服务器实时流式推送进度
 *      - intent_router 分流：闲聊→混元（注入长期记忆）；操控手机→Codex CLI 读 AGENTS.md
 *   2. **HTTP CLI**（`/api/projects/{id}/chat`）[WS 断开时]
 *      - system prompt 拼接到 user 消息，让 Codex 收到完整格式指令
 *   3. **HTTP LLM**（`/api/llm/chat`）[CLI 失败时]
 *      - 服务器注入用户长期记忆 + 近期会话历史（P1+P2 保障）
 *
 * ## 手机控制项目空间（P2 会话连续）
 *   首次调用自动 ensure，创建"手机控制"项目并写入 AGENTS.md；
 *   cliConversationId / lmConversationId 跨请求维护，保持上下文连续。
 */
class ElonServerAIClient(
    context: Context,
    @Suppress("UNUSED_PARAMETER") legacyProjectId: String
) : AIClient {

    private val ctx = context.applicationContext
    private val server = ElonServerClient(ctx, MainAppBridge.serverUrl(ctx))

    /** WS 进度回调，由 SmartResponseGenerator 注入 */
    var onProgress: ((String) -> Unit)? = null

    @Volatile private var balloonProjectId: String? = null
    @Volatile private var wsClient: ProjectChatWsClient? = null
    @Volatile private var cliConversationId: String? = null
    @Volatile private var lmConversationId: String? = null
    private val ensureDone = java.util.concurrent.atomic.AtomicBoolean(false)

    // ── 初始化"手机控制"项目空间 ─────────────────────────────

    private suspend fun ensureBalloonProject(): String? {
        if (ensureDone.get()) return balloonProjectId
        if (!ensureDone.compareAndSet(false, true)) return balloonProjectId

        return withContext(Dispatchers.IO) {
            try {
                val token = MainAppBridge.authToken(ctx)
                    ?: run { ensureDone.set(false); return@withContext null }
                val url = java.net.URL("${MainAppBridge.serverUrl(ctx)}/api/agent-balloon/ensure")
                val conn = (url.openConnection() as java.net.HttpURLConnection).apply {
                    requestMethod = "POST"
                    setRequestProperty("Authorization", "Bearer $token")
                    setRequestProperty("Content-Length", "0")
                    connectTimeout = 10_000; readTimeout = 10_000
                }
                try {
                    if (conn.responseCode == 200) {
                        val pid = org.json.JSONObject(
                            conn.inputStream.bufferedReader().readText()
                        ).optString("project_id")
                        if (pid.isNotBlank()) {
                            balloonProjectId = pid
                            wsClient = ProjectChatWsClient(MainAppBridge.serverUrl(ctx), pid, token)
                                .also { it.ensureConnected() }
                            Log.i(TAG, "✅ balloon ensured=$pid  WS 预热中")
                            return@withContext pid
                        }
                    }
                } finally { conn.disconnect() }
                ensureDone.set(false); null
            } catch (e: Exception) {
                Log.w(TAG, "ensure balloon 失败（非致命）: ${e.message}")
                ensureDone.set(false); null
            }
        }
    }

    // ── chat 主链路 ──────────────────────────────────────────

    override suspend fun chat(messages: List<Message>): String {
        val projectId = ensureBalloonProject()

        // system prompt 拼接工具函数：把 system 内容前置给 Codex，
        // 让脚本生成 prompt、意图分析 prompt 等完整传达。
        val systemCtx = messages.filter { it.role == "system" }
            .joinToString("\n") { it.content }
        val userMsg = messages.lastOrNull { it.role == "user" }?.content
            ?: messages.joinToString("\n") { "${it.role}: ${it.content}" }
        val fullMsgForCli = if (systemCtx.isNotBlank()) "$systemCtx\n\n---\n$userMsg" else userMsg

        // 1. WebSocket 全双工（优先，支持流式进度）
        val ws = wsClient
        if (ws != null && projectId != null) {
            try {
                Log.d(TAG, "→ WS project=$projectId  msg=${fullMsgForCli.take(60)}")
                val reply = ws.chat(
                    message = fullMsgForCli,
                    conversationId = cliConversationId,
                    onProgress = { step -> onProgress?.invoke(step) }
                )
                if (reply.isNotBlank() && reply != "（服务器未返回回复）") {
                    Log.i(TAG, "← WS reply=${reply.take(60)}")
                    return reply
                }
                Log.w(TAG, "WS 返回空，降级 HTTP CLI")
            } catch (e: Exception) {
                Log.w(TAG, "WS 失败（${e.message}），降级 HTTP CLI")
                wsClient = null
                ensureDone.set(false)
            }
        }

        // 2. HTTP CLI 兜底（WS 不可用时）
        if (projectId != null) {
            try {
                Log.d(TAG, "→ HTTP CLI project=$projectId  msg=${fullMsgForCli.take(60)}")
                val (reply, newConvId) = server.chat(
                    projectId = projectId,
                    message = fullMsgForCli,
                    conversationId = cliConversationId,
                    chatOnly = false
                )
                if (newConvId != null) cliConversationId = newConvId
                if (reply.isNotBlank() && reply != "（服务器未返回回复）") return reply
                Log.w(TAG, "HTTP CLI 返回空，降级 LLM")
            } catch (e: Exception) {
                Log.w(TAG, "HTTP CLI 失败（${e.message}），降级 LLM")
            }
        }

        // 3. HTTP LLM 最终兜底（混元，带长期记忆+会话历史注入）
        Log.d(TAG, "→ LLM msgs=${messages.size}  conv=${lmConversationId ?: "<new>"}")
        val (reply, newConvId) = server.lmChat(messages, conversationId = lmConversationId)
        if (newConvId != null) lmConversationId = newConvId
        return reply
    }

    /** 重置会话（新一轮语音对话开始时调用） */
    fun resetConversation() {
        cliConversationId = null
        lmConversationId = null
    }

    /** 释放 WS 连接（Activity onDestroy 时调用） */
    fun releaseWs() {
        wsClient?.disconnect()
        wsClient = null
    }

    companion object { private const val TAG = "ElonServerAIClient" }
}
