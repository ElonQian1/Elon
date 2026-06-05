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
 * 当用户没有自带 API Key 但已在主 UI 登录时，[AIClientFactory] 构造这个适配器。
 * agent 的所有 LLM 调用（闲聊、意图分析、**手机自动化脚本生成**）都通过它完成。
 *
 * 走 POST /api/llm/chat，把 messages 完整送到服务器 LLM：
 *  - system prompt 由调用方控制（闲聊带人设、脚本生成带 JSON 格式说明）
 *  - 服务器同一个 LLM（Codex/Claude）可以并行处理：
 *      主聊天区的项目开发任务（写 Kotlin/Rust 代码）
 *      AND 悬浮球的手机自动化脚本生成（返回 JSON）
 *  - 不触发项目 Codex 工作流，也不覆盖 system prompt
 */
class ElonServerAIClient(
    context: Context,
    private val projectId: String
) : AIClient {

    private val ctx = context.applicationContext
    private val server = ElonServerClient(ctx, MainAppBridge.serverUrl(ctx))

    override suspend fun chat(messages: List<Message>): String {
        Log.d(TAG, "→ /api/llm/chat  msgs=${messages.size}")
        return server.lmChat(messages)
    }

    companion object {
        private const val TAG = "ElonServerAIClient"
    }
}
