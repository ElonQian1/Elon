// infrastructure/ai/LLMClient.kt
// module: infrastructure/ai | layer: infrastructure | role: llm-client
// summary: 大语言模型客户端 - 支持多种 LLM API（通义千问、DeepSeek等）

package com.elon.app.agent.infrastructure.ai

import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

/**
 * 🧠 LLM 客户端
 * 
 * 支持的模型：
 * - 通义千问 (Qwen)
 * - DeepSeek
 * - OpenAI 兼容接口
 * 
 * 使用方式：
 * ```kotlin
 * val client = LLMClient(
 *     provider = LLMProvider.QWEN,
 *     apiKey = "your-api-key"
 * )
 * val response = client.chat("你好")
 * ```
 */
class LLMClient(
    private val provider: LLMProvider = LLMProvider.DEEPSEEK,
    private val apiKey: String = "",  // 需要配置
    private val model: String? = null,  // 可选，使用默认模型
    private val openAICompatibleBaseUrl: String? = null
) {
    companion object {
        private const val TAG = "LLMClient"
        private const val TIMEOUT_MS = 30_000
    }
    
    /**
     * 简单对话
     */
    suspend fun chat(userMessage: String): String {
        return chat(listOf(ChatMessage(role = "user", content = userMessage)))
    }
    
    /**
     * 多轮对话
     */
    suspend fun chat(messages: List<ChatMessage>, systemPrompt: String? = null): String {
        return withContext(Dispatchers.IO) {
            try {
                val allMessages = mutableListOf<ChatMessage>()
                
                // 添加系统提示
                if (systemPrompt != null) {
                    allMessages.add(ChatMessage(role = "system", content = systemPrompt))
                }
                
                allMessages.addAll(messages)
                
                when (provider) {
                    LLMProvider.QWEN -> callQwen(allMessages)
                    LLMProvider.DEEPSEEK -> callDeepSeek(allMessages)
                    LLMProvider.OPENAI_COMPATIBLE -> callOpenAICompatible(allMessages)
                }
            } catch (e: Exception) {
                Log.e(TAG, "LLM 调用失败", e)
                "抱歉，我暂时无法回应。(${e.message})"
            }
        }
    }
    
    /**
     * 调用通义千问
     */
    private fun callQwen(messages: List<ChatMessage>): String {
        val url = URL("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
        val modelName = model ?: "qwen-turbo"
        
        return callOpenAIStyle(url, apiKey, modelName, messages, authHeader = "Authorization" to "Bearer $apiKey")
    }
    
    /**
     * 调用 DeepSeek
     */
    private fun callDeepSeek(messages: List<ChatMessage>): String {
        val url = URL("https://api.deepseek.com/chat/completions")
        val modelName = model ?: "deepseek-chat"
        
        return callOpenAIStyle(url, apiKey, modelName, messages, authHeader = "Authorization" to "Bearer $apiKey")
    }
    
    /**
     * 调用 OpenAI 兼容接口
     */
    private fun callOpenAICompatible(messages: List<ChatMessage>): String {
        val url = openAICompatibleChatUrl()
        val modelName = model ?: "gpt-3.5-turbo"
        
        return callOpenAIStyle(url, apiKey, modelName, messages, authHeader = "Authorization" to "Bearer $apiKey")
    }

    private fun openAICompatibleChatUrl(): URL {
        val base = openAICompatibleBaseUrl?.trim()?.trimEnd('/')
        if (!base.isNullOrBlank()) {
            val path = if (base.endsWith("/chat/completions")) base else "$base/chat/completions"
            return URL(path)
        }
        return URL("https://api.openai.com/v1/chat/completions")
    }
    
    /**
     * 通用 OpenAI 风格 API 调用
     */
    private fun callOpenAIStyle(
        url: URL,
        apiKey: String,
        model: String,
        messages: List<ChatMessage>,
        authHeader: Pair<String, String>
    ): String {
        val connection = url.openConnection() as HttpURLConnection
        
        try {
            connection.apply {
                requestMethod = "POST"
                doOutput = true
                connectTimeout = TIMEOUT_MS
                readTimeout = TIMEOUT_MS
                setRequestProperty("Content-Type", "application/json")
                setRequestProperty(authHeader.first, authHeader.second)
            }
            
            // 构建请求体
            val requestBody = JSONObject().apply {
                put("model", model)
                put("messages", JSONArray().apply {
                    messages.forEach { msg ->
                        put(JSONObject().apply {
                            put("role", msg.role)
                            put("content", msg.content)
                        })
                    }
                })
                put("max_tokens", 500)
                put("temperature", 0.7)
            }
            
            Log.d(TAG, "🚀 请求 LLM: ${provider.name}, model=$model")
            
            // 发送请求
            OutputStreamWriter(connection.outputStream).use { writer ->
                writer.write(requestBody.toString())
                writer.flush()
            }
            
            // 读取响应
            val responseCode = connection.responseCode
            val responseText = if (responseCode == 200) {
                connection.inputStream.bufferedReader().readText()
            } else {
                val error = connection.errorStream?.bufferedReader()?.readText() ?: "Unknown error"
                Log.e(TAG, "❌ API 错误 ($responseCode): $error")
                throw Exception("API error: $responseCode")
            }
            
            // 解析响应
            val json = JSONObject(responseText)
            val content = json
                .getJSONArray("choices")
                .getJSONObject(0)
                .getJSONObject("message")
                .getString("content")
            
            Log.d(TAG, "✅ LLM 响应: ${content.take(100)}...")
            return content.trim()
            
        } finally {
            connection.disconnect()
        }
    }
}

/**
 * LLM 提供商
 */
enum class LLMProvider {
    QWEN,              // 通义千问
    DEEPSEEK,          // DeepSeek
    OPENAI_COMPATIBLE  // OpenAI 兼容接口
}

/**
 * 对话消息
 */
data class ChatMessage(
    val role: String,      // "system", "user", "assistant"
    val content: String
)
