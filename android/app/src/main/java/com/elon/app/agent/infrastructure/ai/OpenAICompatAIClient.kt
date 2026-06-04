// infrastructure/ai/OpenAICompatAIClient.kt
// module: infrastructure/ai | layer: infrastructure | role: openai-compat-adapter
// summary: 把 LLMClient（OpenAI 兼容厂商：DeepSeek / 通义千问 / 任意 OpenAI 兼容接口）适配到 AIClient 接口

package com.elon.app.agent.infrastructure.ai

import com.elon.app.agent.application.AIClient
import com.elon.app.agent.application.Message

/**
 * OpenAI 兼容协议 AI 客户端适配器。
 *
 * 把现有 [LLMClient]（独立 ChatMessage 签名）封装成 [AIClient] 接口，
 * 让上层（ScriptEngine / IntentAnalyzer / SmartResponseGenerator / AIAutonomousEngine）
 * 不需要关心底层是哪个厂商。
 *
 * 当用户在 AgentConfig 里只填了 OpenAI 兼容 Key（比如 DeepSeek），
 * [AIClientFactory] 就会构造这个适配器。
 */
class OpenAICompatAIClient(
    apiKey: String,
    provider: LLMProvider = LLMProvider.DEEPSEEK,
    model: String? = null
) : AIClient {

    private val inner = LLMClient(provider = provider, apiKey = apiKey, model = model)

    override suspend fun chat(messages: List<Message>): String {
        val converted = messages.map { ChatMessage(role = it.role, content = it.content) }
        return inner.chat(converted)
    }
}
