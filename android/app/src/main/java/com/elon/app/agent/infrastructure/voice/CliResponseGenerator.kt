// infrastructure/voice/CliResponseGenerator.kt
// module: infrastructure/voice | layer: infrastructure | role: cli-response-generator
// summary: CLI 模式响应生成器 - 把用户输入发到 elon 服务器，由服务器 CLI AI 回答

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.util.Log
import com.elon.app.agent.application.conversation.IntentAnalysisResult
import com.elon.app.agent.application.conversation.ResponseGenerator
import com.elon.app.agent.application.conversation.StreamingIntentAnalyzer
import com.elon.app.agent.domain.conversation.Emotion
import com.elon.app.agent.domain.conversation.Response
import com.elon.app.agent.domain.conversation.ResponseTier
import com.elon.app.agent.infrastructure.ai.ElonServerClient

/**
 * 🖥️ CLI 模式响应生成器
 *
 * 将用户的语音输入直接发给 elon 服务器，由服务器 CLI（Codex/Claude）回答。
 * 相比 API Key 模式，不需要用户自购 AI API Key，复用 elon 平台的 AI 能力。
 *
 * 使用条件：在 Agent 设置中选择"服务器 CLI"模式并填写 Project ID。
 */
class CliResponseGenerator(
    private val context: Context,
    private val projectId: String,
    serverUrl: String = "http://43.139.149.158:8080"
) : ResponseGenerator {

    companion object {
        private const val TAG = "CliResponseGen"
    }

    private val serverClient = ElonServerClient(context, serverUrl)

    /** 同一语音会话复用一个 conversationId，让服务器保持上下文 */
    private var currentConversationId: String? = null

    /**
     * 进度回调，在 IO 线程调用。
     * 每收到一条服务器 WsMessage（type=progress/step）时回调消息文本，
     * 供 UI 在"思考中"阶段展示具体步骤。
     */
    var onProgress: ((String) -> Unit)? = null

    override suspend fun generate(intent: IntentAnalysisResult): Response {
        Log.d(TAG, "🖥️ CLI 流式请求: ${intent.normalizedInput.take(40)}")
        return try {
            val reply = serverClient.chatStream(
                projectId = projectId,
                message = intent.normalizedInput,
                conversationId = currentConversationId
            ) { type, message ->
                if (type == "progress" || type == "step" || type == "thinking") {
                    onProgress?.invoke(message)
                }
            }
            Log.i(TAG, "🖥️ CLI 回复完成: ${reply.take(60)}")
            Response(
                tier = ResponseTier.NORMAL,
                text = reply,
                emotion = Emotion.HELPFUL
            )
        } catch (e: IllegalStateException) {
            // 未登录
            Log.w(TAG, "未登录: ${e.message}")
            Response(
                tier = ResponseTier.FAST,
                text = "请先在 elon APP 中登录，才能使用服务器 CLI 模式",
                emotion = Emotion.APOLOGETIC
            )
        } catch (e: Exception) {
            Log.w(TAG, "SSE 请求失败，降级到同步 chat", e)
            // 降级：回退到普通 HTTP chat
            try {
                val reply = serverClient.chat(
                    projectId = projectId,
                    message = intent.normalizedInput,
                    conversationId = currentConversationId
                )
                Log.i(TAG, "🖥️ 降级回复: ${reply.take(60)}")
                Response(tier = ResponseTier.NORMAL, text = reply, emotion = Emotion.HELPFUL)
            } catch (e2: Exception) {
                Log.e(TAG, "降级也失败", e2)
                Response(
                    tier = ResponseTier.FAST,
                    text = "服务器响应失败，请检查网络或稍后再试",
                    emotion = Emotion.APOLOGETIC
                )
            }
        }
    }

    /** 重置会话（语音对话结束后调用，下次重新开始新会话） */
    fun resetConversation() {
        currentConversationId = null
    }
}

/**
 * 🔀 CLI 模式透传意图分析器
 *
 * CLI 模式下不需要本地意图分析——服务器 AI 自行理解用户意图。
 * 直接把所有输入标记为"完整的非操作请求"，交由 CliResponseGenerator 处理。
 */
class PassthroughIntentAnalyzer : StreamingIntentAnalyzer {
    override suspend fun analyze(input: String): IntentAnalysisResult {
        return IntentAnalysisResult(
            normalizedInput = input,
            isComplete = true,
            isOperation = false,
            confidence = 1.0f,
            hint = null
        )
    }
}
