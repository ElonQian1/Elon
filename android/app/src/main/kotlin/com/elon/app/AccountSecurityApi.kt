package com.elon.app

import android.content.Context
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.UUID
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

internal data class AccountSecuritySession(
    val id: String,
    val deviceName: String,
    val current: Boolean,
    val trusted: Boolean,
    val lastSeenAt: String?,
    val expiresAt: String,
)

internal data class AccountSecuritySnapshot(
    val passwordEnabled: Boolean,
    val passwordChangedAt: String?,
    val recoveryCodeCount: Int,
    val sessions: List<AccountSecuritySession>,
)

internal class AccountSecurityApi(
    private val context: Context,
    private val http: OkHttpClient = OkHttpClient(),
) {
    suspend fun status(): AccountSecuritySnapshot = withContext(Dispatchers.IO) {
        parseSnapshot(execute(authenticated(Request.Builder().url(url("/api/auth/security")))))
    }

    suspend fun changePassword(currentPassword: String?, newPassword: String) =
        withContext(Dispatchers.IO) {
            val body = JSONObject()
                .put("current_password", currentPassword?.takeIf(String::isNotBlank))
                .put("new_password", newPassword)
                .put("request_id", requestId("password"))
                .put("confirm", true)
            execute(
                authenticated(
                    Request.Builder().url(url("/api/auth/password"))
                        .put(body.toString().toRequestBody(JSON_MEDIA_TYPE)),
                ),
            )
        }

    suspend fun rotateRecoveryCodes(currentPassword: String?): List<String> =
        withContext(Dispatchers.IO) {
            val body = JSONObject()
                .put("current_password", currentPassword?.takeIf(String::isNotBlank))
                .put("request_id", requestId("recovery-codes"))
                .put("confirm", true)
            val result = execute(
                authenticated(
                    Request.Builder().url(url("/api/auth/recovery-codes/rotate"))
                        .post(body.toString().toRequestBody(JSON_MEDIA_TYPE)),
                ),
            ).optJSONObject("result")
            val codes = result?.optJSONArray("codes") ?: return@withContext emptyList()
            (0 until codes.length()).mapNotNull { codes.optString(it).takeIf(String::isNotBlank) }
        }

    suspend fun revokeSession(sessionId: String) = withContext(Dispatchers.IO) {
        execute(
            authenticated(
                Request.Builder().url(url("/api/auth/sessions/${android.net.Uri.encode(sessionId)}"))
                    .delete(),
            ),
        )
    }

    suspend fun revokeOtherSessions(): Int = withContext(Dispatchers.IO) {
        val body = JSONObject().put("confirm", true)
        execute(
            authenticated(
                Request.Builder().url(url("/api/auth/sessions/revoke-others"))
                    .post(body.toString().toRequestBody(JSON_MEDIA_TYPE)),
            ),
        ).optInt("revoked_session_count", 0)
    }

    suspend fun recoverPassword(account: String, recoveryCode: String, newPassword: String) =
        withContext(Dispatchers.IO) {
            val body = JSONObject()
                .put("account", account)
                .put("recovery_code", recoveryCode)
                .put("new_password", newPassword)
                .put("request_id", requestId("recover"))
                .put("client_instance_id", clientInstanceId())
                .put("confirm", true)
            execute(
                Request.Builder().url(url("/api/auth/password/recover"))
                    .post(body.toString().toRequestBody(JSON_MEDIA_TYPE)).build(),
            )
        }

    suspend fun startExternalRecovery(account: String): String = withContext(Dispatchers.IO) {
        val body = JSONObject()
            .put("account", account)
            .put("client_instance_id", clientInstanceId())
        execute(
            Request.Builder().url(url("/api/auth/password/recovery/start"))
                .post(body.toString().toRequestBody(JSON_MEDIA_TYPE)).build(),
        ).optString("message", "邮件或短信恢复尚未配置")
    }

    private fun authenticated(builder: Request.Builder): Request =
        AuthManager.applyAuth(context, builder).build()

    private fun execute(request: Request): JSONObject = http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        val json = runCatching { JSONObject(body.ifBlank { "{}" }) }.getOrDefault(JSONObject())
        if (!response.isSuccessful) {
            error(json.optString("error").ifBlank { "请求失败 (${response.code})" }.take(500))
        }
        json
    }

    private fun parseSnapshot(root: JSONObject): AccountSecuritySnapshot {
        val password = root.optJSONObject("password") ?: JSONObject()
        val recovery = root.optJSONObject("recovery") ?: JSONObject()
        val values = root.optJSONArray("sessions")
        val sessions = if (values == null) emptyList() else (0 until values.length()).mapNotNull { index ->
            values.optJSONObject(index)?.let { value ->
                AccountSecuritySession(
                    id = value.optString("id"),
                    deviceName = value.optString("device_name").ifBlank { "未命名设备" },
                    current = value.optBoolean("current"),
                    trusted = value.optBoolean("trusted_device"),
                    lastSeenAt = value.optionalString("last_seen_at"),
                    expiresAt = value.optString("expires_at"),
                )
            }
        }
        return AccountSecuritySnapshot(
            passwordEnabled = password.optBoolean("enabled"),
            passwordChangedAt = password.optionalString("changed_at"),
            recoveryCodeCount = recovery.optInt("available_code_count"),
            sessions = sessions,
        )
    }

    private fun JSONObject.optionalString(key: String): String? =
        if (!has(key) || isNull(key)) null else optString(key).trim().takeIf(String::isNotBlank)

    private fun requestId(operation: String) = "apk:$operation:${UUID.randomUUID()}"

    private fun clientInstanceId(): String {
        val prefs = AuthManager.prefs(context)
        val existing = prefs.getString(CLIENT_INSTANCE_KEY, null)?.trim()
        if (!existing.isNullOrBlank()) return existing
        return "apk:${UUID.randomUUID()}".also {
            prefs.edit().putString(CLIENT_INSTANCE_KEY, it).apply()
        }
    }

    private fun url(path: String) = ElonApplication.activeServerUrl(context).trimEnd('/') + path

    private companion object {
        val JSON_MEDIA_TYPE = "application/json".toMediaType()
        const val CLIENT_INSTANCE_KEY = "account_security_client_instance_id"
    }
}
