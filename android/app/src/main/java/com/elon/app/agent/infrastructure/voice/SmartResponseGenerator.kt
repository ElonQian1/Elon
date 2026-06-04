// infrastructure/voice/SmartResponseGenerator.kt
// module: infrastructure/voice | layer: infrastructure | role: response-generator
// summary: 智能响应生成器 - 使用 AI 生成自然的对话回复

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.util.Log
import com.elon.app.agent.application.Message
import com.elon.app.agent.application.conversation.IntentAnalysisResult
import com.elon.app.agent.application.conversation.ResponseGenerator
import com.elon.app.agent.domain.conversation.Emotion
import com.elon.app.agent.domain.conversation.Response
import com.elon.app.agent.domain.conversation.ResponseTier
import com.elon.app.agent.infrastructure.ai.AIClientFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * 🎭 智能响应生成器
 * 
 * 根据意图分析结果生成自然的对话回复
 * 
 * 功能：
 * 1. 操作请求 → 生成确认+执行语句
 * 2. 问候语 → 生成友好回应
 * 3. 闲聊 → 调用 AI 生成回复
 * 4. 问答 → 调用 AI 回答问题
 *
 * **重构后**：不再接收 apiKey，通过 [AIClientFactory] 自动选择 AI 链路。
 */
class SmartResponseGenerator(
    context: Context
) : ResponseGenerator {
    
    companion object {
        private const val TAG = "SmartResponseGen"
        
        // 助手人设
        private const val ASSISTANT_PERSONA = """你是一个智能手机助手，名字叫小智。
性格特点：
- 热情友好，语气自然亲切
- 回复简洁，一般不超过30字
- 能帮用户操作手机（打开APP、搜索、发消息等）
- 不确定的事情会诚实说不知道

回复规则：
- 如果用户让你做手机操作，简单确认即可，比如"好的，马上帮你打开"
- 如果用户在闲聊，正常对话就好
- 保持口语化，不要太正式"""
    }
    
    private val aiClient = AIClientFactory.create(context)
    
    override suspend fun generate(intent: IntentAnalysisResult): Response {
        Log.d(TAG, "🎭 [响应生成开始] input=${intent.normalizedInput}, operation=${intent.isOperation}, complete=${intent.isComplete}")
        
        val response = when {
            // 操作请求 → 快速确认
            intent.isOperation -> {
                Log.d(TAG, "🎭 [路由] → 操作确认响应")
                generateOperationResponse(intent)
            }
            
            // 不完整输入 → 追问
            !intent.isComplete -> {
                Log.d(TAG, "🎭 [路由] → 追问响应")
                generateClarificationResponse(intent)
            }
            
            // 其他 → AI 对话
            else -> {
                Log.d(TAG, "🎭 [路由] → AI对话响应")
                generateChatResponse(intent)
            }
        }
        
        Log.i(TAG, "🎭 [响应生成完成] text=${response.text}, tier=${response.tier}")
        return response
    }
    
    /**
     * 生成操作确认响应
     */
    private fun generateOperationResponse(intent: IntentAnalysisResult): Response {
        val input = intent.normalizedInput
        
        // 根据操作类型生成不同的确认语
        val confirmText = when {
            input.contains("打开") -> "好的，马上帮你${input}"
            input.contains("搜索") -> "好的，我来帮你${input}"
            input.contains("发送") || input.contains("发") -> "好的，帮你${input}"
            input.contains("返回") || input.contains("退出") -> "好的，${input}"
            else -> "好的，我来帮你${input}"
        }
        
        Log.i(TAG, "✅ 操作响应: $confirmText")
        
        return Response(
            tier = ResponseTier.FAST,
            text = confirmText,
            emotion = Emotion.HELPFUL,
            requiresAction = true,
            actionDescription = input
        )
    }
    
    /**
     * 生成追问响应
     */
    private fun generateClarificationResponse(intent: IntentAnalysisResult): Response {
        val hint = intent.hint ?: "请继续说完整..."
        
        return Response(
            tier = ResponseTier.INSTANT,
            text = hint,
            emotion = Emotion.CURIOUS,
            requiresAction = false
        )
    }
    
    /**
     * 生成 AI 对话响应
     */
    private suspend fun generateChatResponse(intent: IntentAnalysisResult): Response = withContext(Dispatchers.IO) {
        try {
            val startTime = System.currentTimeMillis()
            
            val messages = listOf(
                Message(role = "system", content = ASSISTANT_PERSONA),
                Message(role = "user", content = intent.normalizedInput)
            )
            
            val response = aiClient.chat(messages)
            val elapsed = System.currentTimeMillis() - startTime
            
            Log.i(TAG, "✅ AI 响应 (${elapsed}ms): $response")
            
            // 判断响应层级
            val tier = when {
                elapsed < 500 -> ResponseTier.FAST
                elapsed < 2000 -> ResponseTier.NORMAL
                else -> ResponseTier.DEEP
            }
            
            Response(
                tier = tier,
                text = response.take(100), // 限制长度
                emotion = detectEmotion(response),
                generationTimeMs = elapsed
            )
        } catch (e: Exception) {
            Log.e(TAG, "AI 响应失败", e)
            
            // 失败时返回友好的默认回复
            Response(
                tier = ResponseTier.FAST,
                text = "抱歉，我没太理解。你可以再说一遍吗？",
                emotion = Emotion.APOLOGETIC
            )
        }
    }
    
    /**
     * 检测回复的情感
     */
    private fun detectEmotion(text: String): Emotion {
        return when {
            text.contains("抱歉") || text.contains("对不起") -> Emotion.APOLOGETIC
            text.contains("哈哈") || text.contains("😄") || text.contains("开心") -> Emotion.HAPPY
            text.contains("好的") || text.contains("没问题") -> Emotion.HELPFUL
            text.contains("？") || text.contains("?") -> Emotion.CURIOUS
            else -> Emotion.NEUTRAL
        }
    }
}
