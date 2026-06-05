// infrastructure/ai/ElonServerAIClient.kt
// module: infrastructure/ai | layer: infrastructure | role: elon-server-adapter
// summary: 把 ElonServerClient 适配到 AIClient 接口，让 agent 在用户没配 API Key 时自动走服务器 CLI

package com.elon.app.agent.infrastructure.ai

import android.content.Context
import android.util.Log
import com.elon.app.agent.application.AIClient
import com.elon.app.agent.application.Message
import com.elon.app.agent.infrastructure.auth.MainAppBridge

/**
 * elon 服务器 LLM 适配器。
 *
 * 走 POST /api/llm/chat：
 *  - system prompt 由调用方完整控制（闲聊带人设、脚本生成带 JSON 格式说明）
 *  - 服务器自动注入用户长期记忆（跨悬浮球+聊天区共享）
 *  - 维护 conversation_id 让服务器保持上下文
 *  - 结束后服务器异步提取新记忆写入数据库
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

    override suspend fun chat(messages: List<Message>): String {
        Log.d(TAG, "→ /api/llm/chat  msgs=${messages.size}  conv=${conversationId ?: "<new>"}")
        val (reply, newConvId) = server.lmChat(messages, conversationId = conversationId)
        if (newConvId != null) conversationId = newConvId
        return reply
    }

    companion object {
        private const val TAG = "ElonServerAIClient"
    }
}
