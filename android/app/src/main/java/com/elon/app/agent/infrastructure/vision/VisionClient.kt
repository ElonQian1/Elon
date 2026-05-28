// infrastructure/vision/VisionClient.kt
// module: infrastructure/vision | layer: infrastructure | role: vision-client
// summary: 多模态 Vision API 客户端，支持多种提供商

package com.elon.app.agent.infrastructure.vision

import android.util.Log
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.HttpURLConnection
import java.net.URL

/**
 * Vision API 客户端接口
 */
interface VisionClient {
    /**
     * 分析图像
     * @param imageBase64 Base64 编码的图片
     * @param prompt 分析提示词
     * @return 分析结果
     */
    suspend fun analyzeImage(imageBase64: String, prompt: String): VisionResult
    
    /**
     * 多模态聊天（文本 + 图像）
     */
    suspend fun chat(
        messages: List<VisionMessage>,
        images: List<String> = emptyList()
    ): VisionResult
}

/**
 * Vision 消息
 */
data class VisionMessage(
    val role: String,  // system, user, assistant
    val content: String,
    val imageBase64: String? = null
)

/**
 * Vision 分析结果
 */
sealed class VisionResult {
    data class Success(
        val content: String,
        val tokensUsed: Int = 0
    ) : VisionResult()
    
    data class Failure(
        val error: String,
        val code: Int = -1
    ) : VisionResult()
}

/**
 * 通义千问 VL (Qwen-VL) 客户端
 * 阿里云多模态大模型，性价比高
 */
class QwenVLClient(
    private val apiKey: String,
    private val model: String = "qwen-vl-plus",
    private val endpoint: String = "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation"
) : VisionClient {
    
    private val gson = Gson()
    
    override suspend fun analyzeImage(imageBase64: String, prompt: String): VisionResult {
        return chat(
            messages = listOf(VisionMessage("user", prompt, imageBase64))
        )
    }
    
    override suspend fun chat(
        messages: List<VisionMessage>,
        images: List<String>
    ): VisionResult = withContext(Dispatchers.IO) {
        try {
            val qwenMessages = messages.map { msg ->
                val content = mutableListOf<QwenContent>()
                
                // 添加图片
                msg.imageBase64?.let {
                    content.add(QwenContent(image = "data:image/jpeg;base64,$it"))
                }
                
                // 添加文本
                content.add(QwenContent(text = msg.content))
                
                QwenMessage(role = msg.role, content = content)
            }
            
            val request = QwenVLRequest(
                model = model,
                input = QwenInput(messages = qwenMessages)
            )
            
            val response = executeRequest(request)
            
            VisionResult.Success(
                content = response.output?.choices?.firstOrNull()?.message?.content
                    ?.filterIsInstance<QwenContent>()
                    ?.firstOrNull { it.text != null }?.text
                    ?: "",
                tokensUsed = response.usage?.totalTokens ?: 0
            )
        } catch (e: Exception) {
            Log.e("QwenVL", "调用失败", e)
            VisionResult.Failure(e.message ?: "未知错误")
        }
    }
    
    private fun executeRequest(request: QwenVLRequest): QwenVLResponse {
        val url = URL(endpoint)
        val connection = url.openConnection() as HttpURLConnection
        connection.apply {
            requestMethod = "POST"
            setRequestProperty("Content-Type", "application/json")
            setRequestProperty("Authorization", "Bearer $apiKey")
            doOutput = true
            connectTimeout = 30000
            readTimeout = 60000
        }
        
        val body = gson.toJson(request)
        Log.d("QwenVL", "Request: ${body.take(500)}...")
        
        connection.outputStream.use { it.write(body.toByteArray()) }
        
        val responseCode = connection.responseCode
        if (responseCode != 200) {
            val error = connection.errorStream?.bufferedReader()?.readText()
            throw Exception("HTTP $responseCode: $error")
        }
        
        val responseText = connection.inputStream.bufferedReader().readText()
        Log.d("QwenVL", "Response: ${responseText.take(500)}...")
        
        return gson.fromJson(responseText, QwenVLResponse::class.java)
    }
}

/**
 * OpenAI GPT-4 Vision 客户端
 */
class GPT4VisionClient(
    private val apiKey: String,
    private val model: String = "gpt-4-vision-preview",
    private val endpoint: String = "https://api.openai.com/v1/chat/completions"
) : VisionClient {
    
    private val gson = Gson()
    
    override suspend fun analyzeImage(imageBase64: String, prompt: String): VisionResult {
        return chat(
            messages = listOf(VisionMessage("user", prompt, imageBase64))
        )
    }
    
    override suspend fun chat(
        messages: List<VisionMessage>,
        images: List<String>
    ): VisionResult = withContext(Dispatchers.IO) {
        try {
            val openaiMessages = messages.map { msg ->
                val content = mutableListOf<OpenAIContent>()
                
                // 添加图片
                msg.imageBase64?.let {
                    content.add(OpenAIContent(
                        type = "image_url",
                        imageUrl = OpenAIImageUrl("data:image/jpeg;base64,$it", "low")
                    ))
                }
                
                // 添加文本
                content.add(OpenAIContent(type = "text", text = msg.content))
                
                OpenAIMessage(role = msg.role, content = content)
            }
            
            val request = OpenAIVisionRequest(
                model = model,
                messages = openaiMessages,
                maxTokens = 2000
            )
            
            val response = executeOpenAIRequest(request)
            
            VisionResult.Success(
                content = response.choices.firstOrNull()?.message?.content ?: "",
                tokensUsed = response.usage?.totalTokens ?: 0
            )
        } catch (e: Exception) {
            Log.e("GPT4V", "调用失败", e)
            VisionResult.Failure(e.message ?: "未知错误")
        }
    }
    
    private fun executeOpenAIRequest(request: OpenAIVisionRequest): OpenAIVisionResponse {
        val url = URL(endpoint)
        val connection = url.openConnection() as HttpURLConnection
        connection.apply {
            requestMethod = "POST"
            setRequestProperty("Content-Type", "application/json")
            setRequestProperty("Authorization", "Bearer $apiKey")
            doOutput = true
            connectTimeout = 30000
            readTimeout = 120000
        }
        
        val body = gson.toJson(request)
        connection.outputStream.use { it.write(body.toByteArray()) }
        
        val responseCode = connection.responseCode
        if (responseCode != 200) {
            val error = connection.errorStream?.bufferedReader()?.readText()
            throw Exception("HTTP $responseCode: $error")
        }
        
        val responseText = connection.inputStream.bufferedReader().readText()
        return gson.fromJson(responseText, OpenAIVisionResponse::class.java)
    }
}

// ============ Qwen-VL API 数据模型 ============

data class QwenVLRequest(
    val model: String,
    val input: QwenInput
)

data class QwenInput(
    val messages: List<QwenMessage>
)

data class QwenMessage(
    val role: String,
    val content: List<QwenContent>
)

data class QwenContent(
    val text: String? = null,
    val image: String? = null
)

data class QwenVLResponse(
    val output: QwenOutput?,
    val usage: QwenUsage?
)

data class QwenOutput(
    val choices: List<QwenChoice>?
)

data class QwenChoice(
    val message: QwenMessage?
)

data class QwenUsage(
    @SerializedName("total_tokens")
    val totalTokens: Int
)

// ============ OpenAI Vision API 数据模型 ============

data class OpenAIVisionRequest(
    val model: String,
    val messages: List<OpenAIMessage>,
    @SerializedName("max_tokens")
    val maxTokens: Int = 2000
)

data class OpenAIMessage(
    val role: String,
    val content: List<OpenAIContent>
)

data class OpenAIContent(
    val type: String,
    val text: String? = null,
    @SerializedName("image_url")
    val imageUrl: OpenAIImageUrl? = null
)

data class OpenAIImageUrl(
    val url: String,
    val detail: String = "low"
)

data class OpenAIVisionResponse(
    val choices: List<OpenAIChoice>,
    val usage: OpenAIUsage?
)

data class OpenAIChoice(
    val message: OpenAIResponseMessage
)

data class OpenAIResponseMessage(
    val content: String
)

data class OpenAIUsage(
    @SerializedName("total_tokens")
    val totalTokens: Int
)
