// infrastructure/network/CloudSyncService.kt
// module: infrastructure/network | layer: infrastructure | role: cloud-sync
// summary: 与云端服务器同步数据的服务

package com.elon.app.agent.infrastructure.network

import android.content.Context
import android.provider.Settings
import android.util.Log
import kotlinx.coroutines.*
import org.json.JSONArray
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

/**
 * 云同步服务
 * 
 * 职责：
 * - 生成并缓存设备唯一ID
 * - 同步设备配置到云端
 * - 上传抓取的评论数据
 * - 获取备用 AI Key
 */
class CloudSyncService(private val context: Context) {
    
    companion object {
        private const val TAG = "CloudSyncService"
        private const val PREFS_NAME = "cloud_sync_prefs"
        private const val KEY_DEVICE_ID = "device_id"
        private const val DEFAULT_SERVER_URL = "http://119.91.19.232:8080"
        private const val TIMEOUT_MS = 10000
    }
    
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    private var cachedDeviceId: String? = null
    
    /**
     * 获取服务器地址
     */
    fun getServerUrl(): String {
        return prefs.getString("server_url", DEFAULT_SERVER_URL) ?: DEFAULT_SERVER_URL
    }
    
    /**
     * 设置服务器地址
     */
    fun setServerUrl(url: String) {
        prefs.edit().putString("server_url", url).apply()
    }
    
    /**
     * 获取设备唯一ID
     * 优先从缓存读取，否则生成新ID
     */
    fun getDeviceId(): String {
        cachedDeviceId?.let { return it }
        
        // 从 SharedPreferences 读取
        prefs.getString(KEY_DEVICE_ID, null)?.let { 
            cachedDeviceId = it
            return it 
        }
        
        // 生成新ID
        val newId = generateDeviceId()
        prefs.edit().putString(KEY_DEVICE_ID, newId).apply()
        cachedDeviceId = newId
        return newId
    }
    
    /**
     * 生成设备唯一标识
     * 使用 Android ID + 设备信息的哈希
     */
    private fun generateDeviceId(): String {
        val androidId = Settings.Secure.getString(
            context.contentResolver,
            Settings.Secure.ANDROID_ID
        ) ?: "unknown"
        
        val deviceInfo = "${androidId}_${android.os.Build.MODEL}_${android.os.Build.MANUFACTURER}"
        val hash = sha256(deviceInfo).take(24)
        
        return "android-$hash"
    }
    
    /**
     * SHA256 哈希
     */
    private fun sha256(input: String): String {
        val bytes = MessageDigest.getInstance("SHA-256").digest(input.toByteArray())
        return bytes.joinToString("") { "%02x".format(it) }
    }
    
    // ============ API 方法 ============
    
    /**
     * 检查服务器是否可用
     */
    suspend fun healthCheck(): Boolean = withContext(Dispatchers.IO) {
        try {
            val url = URL("${getServerUrl()}/health")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 5000
            conn.readTimeout = 5000
            conn.requestMethod = "GET"
            
            val result = conn.responseCode == 200
            conn.disconnect()
            result
        } catch (e: Exception) {
            Log.w(TAG, "Health check failed: ${e.message}")
            false
        }
    }
    
    /**
     * 获取设备配置
     */
    suspend fun getDeviceConfig(): JSONObject? = withContext(Dispatchers.IO) {
        try {
            val response = httpGet("/api/device/${getDeviceId()}/config")
            if (response != null) JSONObject(response) else null
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get device config: ${e.message}")
            null
        }
    }
    
    /**
     * 保存设备配置
     */
    suspend fun saveDeviceConfig(config: JSONObject): Boolean = withContext(Dispatchers.IO) {
        try {
            val body = JSONObject().apply {
                put("device_id", getDeviceId())
                put("device_type", "android")
                put("device_name", android.os.Build.MODEL)
                put("config_json", config)
            }
            
            val response = httpPut("/api/device/${getDeviceId()}/config", body.toString())
            response != null
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save device config: ${e.message}")
            false
        }
    }
    
    /**
     * 批量上传评论
     */
    suspend fun uploadComments(comments: List<CommentData>): UploadResult = withContext(Dispatchers.IO) {
        try {
            val commentsArray = JSONArray()
            comments.forEach { comment ->
                commentsArray.put(JSONObject().apply {
                    put("id", comment.id)
                    put("platform", comment.platform)
                    put("video_url", comment.videoUrl)
                    put("author", comment.author)
                    put("content", comment.content)
                    put("ts", comment.timestamp)
                })
            }
            
            val body = JSONObject().apply {
                put("device_id", getDeviceId())
                put("comments", commentsArray)
            }
            
            val response = httpPost("/api/comments/batch", body.toString())
            if (response != null) {
                val json = JSONObject(response)
                UploadResult(
                    success = true,
                    inserted = json.optInt("inserted", 0),
                    updated = json.optInt("updated", 0)
                )
            } else {
                UploadResult(success = false)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to upload comments: ${e.message}")
            UploadResult(success = false, error = e.message)
        }
    }
    
    /**
     * 获取备用 AI Key
     */
    suspend fun getAIFallback(): AIFallbackConfig? = withContext(Dispatchers.IO) {
        try {
            val response = httpGet("/api/ai-config/fallback")
            if (response != null) {
                val json = JSONObject(response)
                if (json.optBoolean("has_fallback", false)) {
                    AIFallbackConfig(
                        provider = json.optString("provider"),
                        key = json.optString("key")
                    )
                } else null
            } else null
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get AI fallback: ${e.message}")
            null
        }
    }
    
    // ============ HTTP 工具方法 ============
    
    private fun httpGet(endpoint: String): String? {
        val url = URL("${getServerUrl()}$endpoint")
        val conn = url.openConnection() as HttpURLConnection
        conn.connectTimeout = TIMEOUT_MS
        conn.readTimeout = TIMEOUT_MS
        conn.requestMethod = "GET"
        
        return try {
            if (conn.responseCode == 200) {
                conn.inputStream.bufferedReader().readText()
            } else null
        } finally {
            conn.disconnect()
        }
    }
    
    private fun httpPost(endpoint: String, body: String): String? {
        return httpRequest("POST", endpoint, body)
    }
    
    private fun httpPut(endpoint: String, body: String): String? {
        return httpRequest("PUT", endpoint, body)
    }
    
    private fun httpRequest(method: String, endpoint: String, body: String): String? {
        val url = URL("${getServerUrl()}$endpoint")
        val conn = url.openConnection() as HttpURLConnection
        conn.connectTimeout = TIMEOUT_MS
        conn.readTimeout = TIMEOUT_MS
        conn.requestMethod = method
        conn.doOutput = true
        conn.setRequestProperty("Content-Type", "application/json")
        
        return try {
            OutputStreamWriter(conn.outputStream).use { writer ->
                writer.write(body)
                writer.flush()
            }
            
            if (conn.responseCode in 200..299) {
                conn.inputStream.bufferedReader().readText()
            } else {
                Log.w(TAG, "HTTP $method $endpoint failed: ${conn.responseCode}")
                null
            }
        } finally {
            conn.disconnect()
        }
    }
}

/**
 * 评论数据
 */
data class CommentData(
    val id: String,
    val platform: String,
    val videoUrl: String?,
    val author: String,
    val content: String,
    val timestamp: Long?
)

/**
 * 上传结果
 */
data class UploadResult(
    val success: Boolean,
    val inserted: Int = 0,
    val updated: Int = 0,
    val error: String? = null
)

/**
 * AI 备用配置
 */
data class AIFallbackConfig(
    val provider: String,
    val key: String
)
