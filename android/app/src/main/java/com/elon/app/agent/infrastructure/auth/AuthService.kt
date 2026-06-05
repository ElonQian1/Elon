// infrastructure/auth/AuthService.kt
// module: infrastructure/auth | layer: infrastructure | role: 认证服务
// summary: 用户登录、注册、Token 管理

package com.elon.app.agent.infrastructure.auth

import android.content.Context
import android.util.Log
import com.elon.app.AuthManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

/**
 * 认证服务
 * 管理用户登录状态、Token 存储
 */
class AuthService(private val context: Context) {
    
    companion object {
        private const val TAG = "AuthService"
        private const val PREFS_NAME = "auth_prefs"
        private const val KEY_TOKEN = "auth_token"
        private const val KEY_USER_ID = "user_id"
        private const val KEY_USERNAME = "username"
        private const val KEY_NICKNAME = "nickname"
        private const val DEFAULT_SERVER_URL = "http://119.91.19.232:8080"
        private const val TIMEOUT_MS = 15000
    }
    
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    
    /**
     * 获取服务器地址（统一使用主 UI 配置的 elon 服务器）
     */
    fun getServerUrl(): String = MainAppBridge.serverUrl(context)
    
    /**
     * 设置服务器地址
     */
    fun setServerUrl(url: String) {
        prefs.edit().putString("server_url", url).apply()
    }
    
    /**
     * 检查是否已登录（复用主 UI 登录态）
     */
    fun isLoggedIn(): Boolean = AuthManager.isLoggedIn(context)

    /**
     * 获取存储的 Token（复用主 UI 登录态）
     */
    fun getToken(): String? = AuthManager.token(context)

    /**
     * 获取当前用户信息（来自主 UI AuthManager，agent 与主 UI 共享同一账号）
     */
    fun getCurrentUser(): UserInfo? {
        val uid = AuthManager.userId(context) ?: return null
        val name = AuthManager.account(context) ?: AuthManager.displayName(context)
        return UserInfo(
            id = uid.toIntOrNull() ?: 0,
            username = name,
            nickname = AuthManager.nickname(context)
        )
    }

    /**
     * 清除登录状态（登出）——统一清除主 UI 登录态
     */
    fun logout() {
        AuthManager.clear(context)
        Log.i(TAG, "User logged out")
    }
    
    /**
     * 注册新用户
     */
    suspend fun register(
        username: String,
        password: String,
        nickname: String? = null
    ): AuthResult = withContext(Dispatchers.IO) {
        try {
            val body = JSONObject().apply {
                put("username", username)
                put("password", password)
                nickname?.let { put("nickname", it) }
            }
            
            val response = httpPost("/api/auth/register", body.toString())
            parseAuthResponse(response)
        } catch (e: Exception) {
            Log.e(TAG, "Register failed: ${e.message}")
            AuthResult(success = false, message = "注册失败: ${e.message}")
        }
    }
    
    /**
     * 用户登录
     */
    suspend fun login(username: String, password: String): AuthResult = withContext(Dispatchers.IO) {
        try {
            val body = JSONObject().apply {
                put("username", username)
                put("password", password)
            }
            
            val response = httpPost("/api/auth/login", body.toString())
            parseAuthResponse(response)
        } catch (e: Exception) {
            Log.e(TAG, "Login failed: ${e.message}")
            AuthResult(success = false, message = "登录失败: ${e.message}")
        }
    }
    
    /**
     * 验证 Token 有效性
     */
    suspend fun verifyToken(): AuthResult = withContext(Dispatchers.IO) {
        val token = getToken() ?: return@withContext AuthResult(
            success = false,
            message = "未登录"
        )
        
        try {
            val response = httpGet("/api/auth/me", token)
            val json = JSONObject(response)
            
            if (json.optBoolean("success", false)) {
                val userJson = json.optJSONObject("user")
                if (userJson != null) {
                    val user = UserInfo(
                        id = userJson.optInt("id"),
                        username = userJson.optString("username"),
                        nickname = userJson.optString("nickname", null)
                    )
                    AuthResult(success = true, user = user)
                } else {
                    AuthResult(success = false, message = "用户信息无效")
                }
            } else {
                // Token 无效，清除登录状态
                logout()
                AuthResult(success = false, message = json.optString("message", "Token 无效"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "Token verification failed: ${e.message}")
            AuthResult(success = false, message = "验证失败: ${e.message}")
        }
    }
    
    /**
     * 解析认证响应（兼容主服务器格式，成功后落到主 UI 登录态）
     */
    private fun parseAuthResponse(response: String): AuthResult {
        val json = JSONObject(response)
        val message = json.optString("message", null)

        val token = json.optString("token", "").ifBlank { null }
        val userJson = json.optJSONObject("user")
        if (token != null && userJson != null) {
            val uid = userJson.optString("id", "").trim()
            val name = userJson.optString("account", "").ifBlank {
                userJson.optString("username", "")
            }
            val nickname = userJson.optString("nickname", "").ifBlank { null }
            if (uid.isNotBlank()) {
                // 统一落到主 UI 登录态（"elon" prefs），与主账号共享
                AuthManager.saveSession(context, token, uid, name.ifBlank { null }, nickname, null)
                Log.i(TAG, "Auth success: $name")
                val user = UserInfo(id = uid.toIntOrNull() ?: 0, username = name, nickname = nickname)
                return AuthResult(success = true, message = message, token = token, user = user)
            }
        }

        return AuthResult(success = false, message = message ?: "登录失败")
    }
    
    // ============ HTTP 工具方法 ============
    
    private fun httpPost(endpoint: String, body: String): String {
        val url = URL("${getServerUrl()}$endpoint")
        val conn = url.openConnection() as HttpURLConnection
        conn.connectTimeout = TIMEOUT_MS
        conn.readTimeout = TIMEOUT_MS
        conn.requestMethod = "POST"
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
                val error = conn.errorStream?.bufferedReader()?.readText() ?: "Unknown error"
                throw Exception("HTTP ${conn.responseCode}: $error")
            }
        } finally {
            conn.disconnect()
        }
    }
    
    private fun httpGet(endpoint: String, token: String? = null): String {
        val url = URL("${getServerUrl()}$endpoint")
        val conn = url.openConnection() as HttpURLConnection
        conn.connectTimeout = TIMEOUT_MS
        conn.readTimeout = TIMEOUT_MS
        conn.requestMethod = "GET"
        token?.let { conn.setRequestProperty("Authorization", "Bearer $it") }
        
        return try {
            if (conn.responseCode in 200..299) {
                conn.inputStream.bufferedReader().readText()
            } else {
                val error = conn.errorStream?.bufferedReader()?.readText() ?: "Unknown error"
                throw Exception("HTTP ${conn.responseCode}: $error")
            }
        } finally {
            conn.disconnect()
        }
    }
}

/**
 * 用户信息
 */
data class UserInfo(
    val id: Int,
    val username: String,
    val nickname: String?
)

/**
 * 认证结果
 */
data class AuthResult(
    val success: Boolean,
    val message: String? = null,
    val token: String? = null,
    val user: UserInfo? = null
)
