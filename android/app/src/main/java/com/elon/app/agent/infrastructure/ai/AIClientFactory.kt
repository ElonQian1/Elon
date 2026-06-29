// infrastructure/ai/AIClientFactory.kt
// module: infrastructure/ai | layer: infrastructure | role: ai-client-factory
// summary: 统一 AI 客户端工厂 - 按优先级自动选择 Hunyuan / OpenAI 兼容 / 服务器 CLI

package com.elon.app.agent.infrastructure.ai

import android.content.Context
import android.util.Log
import com.elon.app.agent.AgentConfigActivity
import com.elon.app.agent.application.AIClient
import com.elon.app.agent.application.Message
import com.elon.app.agent.infrastructure.auth.MainAppBridge

/**
 * 🏭 AI 客户端工厂。
 *
 * 决策优先级：
 *   1. 用户在 AgentConfig 里填了 **腾讯混元** API Key → [HunyuanAIClient]
 *   2. 用户在 AgentConfig 里填了 **OpenAI 兼容** API Key（DeepSeek/通义千问/...）→ [OpenAICompatAIClient]
 *   3. 用户已在主 UI **登录** 且选择了 **当前项目** → [ElonServerAIClient]（走服务器 CLI）
 *   4. 都没有 → 返回 [UnconfiguredAIClient]，调用 chat() 时抛出友好错误
 *
 * 这是 agent 子系统对外的**唯一** AI 入口，
 * 所有 ScriptEngine / IntentAnalyzer / AIAutonomousEngine / SmartResponseGenerator
 * 都通过这个工厂拿到 [AIClient]，不再硬编码 [HunyuanAIClient]。
 */
object AIClientFactory {
    private const val TAG = "AIClientFactory"

    /**
     * 悬浮球 ElonServerAIClient 单例缓存。
     *
     * 使用单例的原因：
     *   - `cliConversationId` / `lmConversationId` / `balloonProjectId` 存在实例字段里
     *   - ScriptEngine / IntentAnalyzer / SmartResponseGenerator 都调用 create()，
     *     若每次 new 出不同实例，会话 ID 永远无法跨调用维护
     *   - 单例保证整个进程共用同一个会话 ID，实现真正的会话连续（P2）
     */
    @Volatile
    private var cachedElonServerClient: ElonServerAIClient? = null

    /**
     * 根据当前配置和登录态构造 [AIClient]。
     * **永远不会抛异常**；如果什么都没配，返回的是 [UnconfiguredAIClient]，
     * 会在真正发起 chat 时给出可读的错误。
     */
    fun create(context: Context): AIClient {
        val ctx = context.applicationContext
        val cfg = AgentConfigActivity.getAgentConfig(ctx)
        val token = MainAppBridge.authToken(ctx)
        val projectId = MainAppBridge.effectiveCliProjectId(ctx)

        fun serverCliClient(): AIClient? {
            if (token.isNullOrBlank() || projectId.isNullOrBlank()) return null
            Log.i(TAG, "→ 选择 ElonServer CLI（project=$projectId）")
            return cachedElonServerClient
                ?: ElonServerAIClient(ctx, projectId).also { cachedElonServerClient = it }
        }

        fun apiKeyClient(): AIClient? {
            if (cfg.hunyuanApiKey.isNotBlank()) {
                Log.i(TAG, "→ 选择 Hunyuan（用户自带 Key）")
                return HunyuanAIClient(cfg.hunyuanApiKey)
            }

            if (cfg.openaiApiKey.isNotBlank()) {
                Log.i(TAG, "→ 选择 OpenAICompat（用户自带 Key）")
                return OpenAICompatAIClient(
                    apiKey = cfg.openaiApiKey,
                    provider = LLMProvider.OPENAI_COMPATIBLE,
                    model = cfg.openaiModel.takeIf { it.isNotBlank() },
                    apiBase = cfg.openaiApiBase.takeIf { it.isNotBlank() }
                )
            }

            return null
        }

        cfg.voiceModeOrder.forEach { mode ->
            when (mode) {
                AgentConfigActivity.VOICE_MODE_APIKEY -> apiKeyClient()?.let { return it }
                AgentConfigActivity.VOICE_MODE_CLI -> serverCliClient()?.let { return it }
            }
        }

        apiKeyClient()?.let { return it }
        serverCliClient()?.let { return it }

        // 都没有 → 返回 NoOp，让调用方在 chat 时收到友好错误
        Log.w(TAG, "→ 无可用 AI（未登录服务器，也没有配置 API Key）")
        return UnconfiguredAIClient(reason = describeMissingConfig(token.isNullOrBlank(), projectId.isNullOrBlank()))
    }

    /** 是否有可用 AI（不抛异常）。用于 UI 状态显示。 */
    fun hasAvailable(context: Context): Boolean {
        return create(context) !is UnconfiguredAIClient
    }

    /** 当前选择的链路描述（UI 显示）。 */
    fun describe(context: Context): String {
        return when (create(context)) {
            is HunyuanAIClient -> "腾讯混元（自带 Key）"
            is OpenAICompatAIClient -> "OpenAI 兼容（自带 Key）"
            is ElonServerAIClient -> "elon 服务器 CLI（${MainAppBridge.effectiveCliProjectId(context).orEmpty()}）"
            is UnconfiguredAIClient -> "未配置"
            else -> "未知"
        }
    }

    private fun describeMissingConfig(noToken: Boolean, noProject: Boolean): String = buildString {
        append("agent 没有可用的 AI：")
        when {
            noToken -> append("请在主页面登录账号；或在 Agent 配置里填写 API Key。")
            noProject -> append("请在主页面选择一个项目；或在 Agent 配置里填写 API Key。")
            else -> append("请在 Agent 配置里填写 API Key。")
        }
    }
}

/**
 * 占位实现：在没有任何可用 AI 的时候用，
 * 调用 [chat] 时直接抛 [IllegalStateException]，错误信息对用户友好。
 */
class UnconfiguredAIClient(private val reason: String) : AIClient {
    override suspend fun chat(messages: List<Message>): String {
        throw IllegalStateException(reason)
    }
}
