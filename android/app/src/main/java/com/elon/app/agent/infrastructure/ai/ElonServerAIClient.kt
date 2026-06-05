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
 * elon 服务器 CLI 适配器（兜底链路）。
 *
 * 当用户没有自带 API Key 但已在主 UI 登录并选择了项目时，
 * [AIClientFactory] 构造这个适配器。
 * agent 的所有 LLM 调用就会发到 `/api/projects/{id}/chat`，
 * 由服务器上的 Codex / Claude / Copilot CLI 代为思考。
 *
 * 注意：服务器 CLI 自己管会话上下文，所以这里只发送最后一条 user 消息，
 * system prompt 完全忽略（服务器有自己的系统人设）。
 */
class ElonServerAIClient(
    context: Context,
    private val projectId: String
) : AIClient {

    private val ctx = context.applicationContext
    private val server = ElonServerClient(ctx, MainAppBridge.serverUrl(ctx))

    /**
     * 维护单一会话 id：第一次调用由服务器创建，后续复用。
     * 这样 agent 在同一个 ScriptEngine / IntentAnalyzer 内的多次 chat
     * 能让服务器知道是同一段对话。
     */
    @Volatile
    private var conversationId: String? = null

    override suspend fun chat(messages: List<Message>): String {
        val userMsg = messages.lastOrNull { it.role == "user" }?.content
            ?: messages.joinToString("\n") { "${it.role}: ${it.content}" }

        Log.d(TAG, "→ server CLI chat (project=$projectId, conv=${conversationId ?: "<new>"})")
        // agent 子系统（闲聊 / 意图分析 / 生成手机脚本）借用服务器 AI 的对话能力，
        // 始终走轻量 chat，绝不触发服务器项目 Codex 开发工作流（避免超时/报错）。
        val reply = server.chat(projectId, userMsg, conversationId, chatOnly = true)
        // ElonServerClient.chat 当前只返回 reply 文本；如果未来返回 conversation_id，
        // 在那里把 conversationId 回填到这里即可（保持单调会话）。
        return reply
    }

    companion object {
        private const val TAG = "ElonServerAIClient"
    }
}
