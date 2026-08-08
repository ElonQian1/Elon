package com.elon.app

import android.content.Context
import android.net.Uri
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.UUID

internal data class AiProviderNode(
    val id: String,
    val label: String,
    val online: Boolean,
)

internal data class AiProviderLoginAttempt(
    val loginId: String,
    val providerId: String,
    val state: String,
    val verificationUrl: String?,
    val userCode: String?,
    val authUrl: String?,
    val remoteCompatible: Boolean,
    val recovered: Boolean,
    val error: String?,
    val errorCode: String?,
) {
    val active: Boolean get() = state == "starting" || state == "waiting_for_user"
    val retryable: Boolean get() = state == "failed" || state == "canceled" || state == "expired"
}

internal data class AiProviderAccount(
    val id: String,
    val label: String,
    val implementationState: String,
    val remoteLoginSupported: Boolean,
    val logoutSupported: Boolean,
    val reason: String?,
    val cliRunnable: Boolean,
    val cliLoggedIn: Boolean?,
    val cliDetail: String?,
    val vaultBackupSupported: Boolean,
    val activeLogin: AiProviderLoginAttempt?,
)

internal data class OpenAiChatKitCapability(
    val configured: Boolean,
    val message: String,
)

internal class AiProviderAccountsApi(
    private val context: Context,
    private val http: OkHttpClient,
) {
    fun fetchNodes(): List<AiProviderNode> = fetchProjectCreateNodes(
        http = http,
        serverUrl = ElonApplication.activeServerUrl(context).trimEnd('/'),
        ctx = context,
    ).map { node ->
        AiProviderNode(node.nodeId, node.displayName, node.online)
    }

    fun fetchAccounts(nodeId: String): List<AiProviderAccount> {
        val root = executeJson(authenticated(Request.Builder().url(providerUrl(nodeId))))
        val providers = root.optJSONArray("providers") ?: return emptyList()
        return (0 until providers.length()).mapNotNull { index ->
            providers.optJSONObject(index)?.let(::parseProvider)
        }
    }

    fun startLogin(nodeId: String, providerId: String): AiProviderLoginAttempt {
        val flow = if (providerId == "codex_cli") "device_code" else "agent"
        val body = JSONObject()
            .put("flow", flow)
            .put("request_id", "apk:${UUID.randomUUID()}")
            .toString()
            .toRequestBody(JSON_MEDIA_TYPE)
        val request = Request.Builder()
            .url(providerUrl(nodeId, providerId, "login"))
            .post(body)
        return parseAttempt(executeJson(authenticated(request)).getJSONObject("attempt"))
    }

    fun loginStatus(
        nodeId: String,
        providerId: String,
        loginId: String,
    ): AiProviderLoginAttempt {
        val request = Request.Builder().url(
            providerUrl(nodeId, providerId, "logins", loginId),
        )
        return parseAttempt(executeJson(authenticated(request)).getJSONObject("attempt"))
    }

    fun cancelLogin(nodeId: String, providerId: String, loginId: String) {
        val request = Request.Builder()
            .url(providerUrl(nodeId, providerId, "logins", loginId, "cancel"))
            .post(EMPTY_JSON)
        executeJson(authenticated(request))
    }

    fun logout(nodeId: String, providerId: String) {
        val request = Request.Builder()
            .url(providerUrl(nodeId, providerId, "logout"))
            .post(EMPTY_JSON)
        executeJson(authenticated(request))
    }

    fun diagnosticsSummary(nodeId: String): String {
        val root = executeJson(authenticated(Request.Builder().url(providerUrl(nodeId, "diagnostics"))))
        val attempts = root.optJSONArray("latest_attempts")
        var retryable = 0
        if (attempts != null) {
            for (index in 0 until attempts.length()) {
                if (attempts.optJSONObject(index)?.optBoolean("retryable") == true) retryable += 1
            }
        }
        val journal = root.optJSONObject("journal")
        val hours = journal?.optInt("retention_hours", 24) ?: 24
        return "脱敏日志保留 ${hours} 小时；最近可重试任务 ${retryable} 个。验证码、授权地址和厂商 token 均不进入诊断。"
    }

    fun fetchChatKitCapability(): OpenAiChatKitCapability {
        val root = executeJson(authenticated(Request.Builder().url(chatKitUrl("capability"))))
        return OpenAiChatKitCapability(
            configured = root.optBoolean("configured", false),
            message = root.optString("message").ifBlank { "管理员尚未配置 OpenAI ChatKit API 服务。" },
        )
    }

    fun createChatKitSession(): String {
        val request = Request.Builder()
            .url(chatKitUrl("session"))
            .post(EMPTY_JSON)
        return executeJson(authenticated(request))
            .optString("client_secret")
            .takeIf(String::isNotBlank)
            ?: error("服务器没有返回 ChatKit 会话")
    }

    private fun authenticated(builder: Request.Builder): Request =
        AuthManager.applyAuth(context, builder).build()

    private fun providerUrl(nodeId: String, vararg tail: String): String {
        val base = ElonApplication.activeServerUrl(context).trimEnd('/')
        val suffix = tail.joinToString("/") { Uri.encode(it) }
        return buildString {
            append(base)
            append("/api/pc-relay/")
            append(Uri.encode(nodeId))
            append("/api/ai-provider-accounts")
            if (suffix.isNotBlank()) append('/').append(suffix)
        }
    }

    private fun chatKitUrl(tail: String): String =
        ElonApplication.activeServerUrl(context).trimEnd('/') +
            "/api/openai-chatkit/" + Uri.encode(tail)

    private fun executeJson(request: Request): JSONObject =
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) {
                val message = runCatching {
                    JSONObject(body).optString("error").ifBlank { body }
                }.getOrDefault(body).ifBlank { "HTTP ${response.code}" }
                error(message.take(500))
            }
            JSONObject(body.ifBlank { "{}" })
        }

    private fun parseProvider(value: JSONObject): AiProviderAccount {
        val cli = value.optJSONObject("cli")
        return AiProviderAccount(
            id = value.optString("id"),
            label = value.optString("label"),
            implementationState = value.optString("implementation_state"),
            remoteLoginSupported = value.optBoolean("remote_login_supported", false),
            logoutSupported = value.optBoolean("logout_supported", false),
            reason = value.optionalString("reason"),
            cliRunnable = cli?.optBoolean("runnable", false) ?: false,
            cliLoggedIn = cli?.let {
                if (it.has("logged_in") && !it.isNull("logged_in")) it.optBoolean("logged_in")
                else null
            },
            cliDetail = cli?.optionalString("detail") ?: cli?.optionalString("reason"),
            vaultBackupSupported = value.optJSONObject("credential_vault")
                ?.optBoolean("backup_supported", false) ?: false,
            activeLogin = value.optJSONObject("active_login")?.let(::parseAttempt),
        )
    }

    private fun parseAttempt(value: JSONObject): AiProviderLoginAttempt = AiProviderLoginAttempt(
        loginId = value.optString("login_id"),
        providerId = value.optString("provider_id"),
        state = value.optString("state"),
        verificationUrl = value.optionalString("verification_url"),
        userCode = value.optionalString("user_code"),
        authUrl = value.optionalString("auth_url"),
        remoteCompatible = value.optBoolean("remote_compatible", false),
        recovered = value.optBoolean("recovered", false),
        error = value.optionalString("error"),
        errorCode = value.optionalString("error_code"),
    )

    private fun JSONObject.optionalString(key: String): String? =
        if (!has(key) || isNull(key)) null else optString(key).trim().takeIf(String::isNotBlank)

    private companion object {
        val JSON_MEDIA_TYPE = "application/json".toMediaType()
        val EMPTY_JSON = "{}".toRequestBody(JSON_MEDIA_TYPE)
    }
}
