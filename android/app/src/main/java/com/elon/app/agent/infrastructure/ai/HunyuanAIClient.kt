// infrastructure/ai/HunyuanAIClient.kt
package com.elon.app.agent.infrastructure.ai

import android.util.Log
import com.elon.app.agent.application.AIClient
import com.elon.app.agent.application.Message
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.HttpURLConnection
import java.net.URL

/**
 * 混元 AI 客户端
 */
class HunyuanAIClient(
    private val apiKey: String,
    private val endpoint: String = "https://api.hunyuan.cloud.tencent.com/v1/chat/completions"
) : AIClient {
    
    private val gson = Gson()
    
    override suspend fun chat(messages: List<Message>): String = withContext(Dispatchers.IO) {
        try {
            val request = HunyuanRequest(
                model = "hunyuan-lite",
                messages = messages.map { 
                    HunyuanMessage(role = it.role, content = it.content) 
                },
                temperature = 0.7,
                maxTokens = 2000
            )
            
            val requestBody = gson.toJson(request)
            Log.d("HunyuanAI", "Request: $requestBody")
            
            val url = URL(endpoint)
            val connection = url.openConnection() as HttpURLConnection
            connection.apply {
                requestMethod = "POST"
                setRequestProperty("Content-Type", "application/json")
                setRequestProperty("Authorization", "Bearer $apiKey")
                doOutput = true
            }
            
            // 发送请求
            connection.outputStream.use { os ->
                os.write(requestBody.toByteArray())
            }
            
            // 读取响应
            val responseCode = connection.responseCode
            if (responseCode != 200) {
                val error = connection.errorStream?.bufferedReader()?.readText()
                throw Exception("HTTP $responseCode: $error")
            }
            
            val responseText = connection.inputStream.bufferedReader().readText()
            Log.d("HunyuanAI", "Response: $responseText")
            
            val response = gson.fromJson(responseText, HunyuanResponse::class.java)
            response.choices.firstOrNull()?.message?.content
                ?: throw Exception("AI 返回空响应")
            
        } catch (e: Exception) {
            Log.e("HunyuanAI", "调用失败", e)
            throw e
        }
    }
}

// Hunyuan API 数据模型
data class HunyuanRequest(
    val model: String,
    val messages: List<HunyuanMessage>,
    val temperature: Double = 0.7,
    @SerializedName("max_tokens")
    val maxTokens: Int = 2000
)

data class HunyuanMessage(
    val role: String,
    val content: String
)

data class HunyuanResponse(
    val choices: List<Choice>
)

data class Choice(
    val message: HunyuanMessage
)
