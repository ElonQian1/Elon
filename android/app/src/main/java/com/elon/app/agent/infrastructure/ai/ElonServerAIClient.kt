// infrastructure/ai/ElonServerAIClient.kt
// module: infrastructure/ai | layer: infrastructure | role: elon-server-adapter
// summary: 把 ElonServerClient 适配到 AIClient 接口，让 agent 在用户没配 API Key 时自动走服务器 LLM

package com.elon.app.agent.infrastructure.ai

import android.content.Context
import android.util.Log
import com.elon.app.agent.application.AIClient
import com.elon.app.agent.application.Message
import com.elon.app.agent.infrastructure.auth.MainAppBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * elon 服务器 LLM 适配器。
 *
 * 走 POST /api/llm/chat（混元），把完整 messages 传给服务器：
 *  - system prompt 由调用方控制（闲聊带人设、脚本生成带 JSON 格式说明）
 *  - 服务器注入用户长期记忆 + 近期会话历史后转发给 LLM
 *  - 结果返回后服务器异步提取新记忆写入数据库
 *
 * 初始化时懒调用 /api/agent-balloon/ensure 为用户在服务器上创建"手机控制"
 * 专属项目空间（幂等），后续请求带 conversationId 保持会话连续。
 */
class ElonServerAIClient(
    context: Context,
    @Suppress("UNUSED_PARAMETER") projectId: String   // 保留参数兼容 AIClientFactory，内部不用
) : AIClient {

    private val ctx = context.applicationContext
    private val server = ElonServerClient(ctx, MainAppBridge.serverUrl(ctx))

    /** 会话 ID，第一次由服务器生成，后续请求带上保持上下文 */
    @Volatile
    private var conversationId: String? = null

    /** 只请求一次 ensure，避免每次 chat 都发 HTTP */
    private val ensureDone = java.util.concurrent.atomic.AtomicBoolean(false)

    /** 确保服务器"手机控制"项目空间存在（懒初始化，幂等）。 */
    private suspend fun ensureBalloonProject() {
        if (!ensureDone.compareAndSet(false, true)) return
        withContext(Dispatchers.IO) {
            try {
                val token = MainAppBridge.authToken(ctx) ?: return@withContext
                val url = java.net.URL("${MainAppBridge.serverUrl(ctx)}/api/agent-balloon/ensure")
                val conn = (url.openConnection() as java.net.HttpURLConnection).apply {
                    requestMethod = "POST"
                    setRequestProperty("Authorization", "Bearer $token")
                    setRequestProperty("Content-Length", "0")
                    connectTimeout = 10_000
                    readTimeout = 10_000
                }
                try {
                    if (conn.responseCode == 200) {
                        val resp = conn.inputStream.bufferedReader().readText()
                        Log.d(TAG, "balloon project ensured: $resp")
                    }
                } finally {
                    conn.disconnect()
                }
            } catch (e: Exception) {
                Log.w(TAG, "ensure balloon project failed (non-fatal): ${e.message}")
                ensureDone.set(false)   // 下次重试
            }
        }
    }

    override suspend fun chat(messages: List<Message>): String {
        ensureBalloonProject()
        Log.d(TAG, "→ /api/llm/chat  msgs=${messages.size}  conv=${conversationId ?: "<new>"}")
        val (reply, newConvId) = server.lmChat(messages, conversationId = conversationId)
        if (newConvId != null) conversationId = newConvId
        return reply
    }

    companion object {
        private const val TAG = "ElonServerAIClient"
    }
}
