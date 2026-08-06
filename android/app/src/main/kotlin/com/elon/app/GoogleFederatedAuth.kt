package com.elon.app

import android.app.Activity
import androidx.credentials.CredentialManager
import androidx.credentials.CustomCredential
import androidx.credentials.GetCredentialRequest
import com.google.android.libraries.identity.googleid.GetGoogleIdOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

internal data class LinkedLoginIdentity(
    val id: String,
    val provider: String,
    val email: String?,
    val displayName: String?,
)

internal class GoogleFederatedAuth(
    private val activity: Activity,
    private val http: OkHttpClient = OkHttpClient(),
) {
    suspend fun authenticate(mode: String): JSONObject {
        require(mode == "login" || mode == "bind")
        val provider = withContext(Dispatchers.IO) { googleProvider() }
        check(provider.optBoolean("configured") && provider.optString("client_id").isNotBlank()) {
            "Google 登录等待管理员配置客户端 ID"
        }
        val challenge = withContext(Dispatchers.IO) { createChallenge(mode) }
        val idToken = requestIdToken(
            provider.getString("client_id"),
            challenge.getString("nonce"),
        )
        return withContext(Dispatchers.IO) {
            complete(challenge.getString("id"), idToken)
        }
    }

    suspend fun identities(): List<LinkedLoginIdentity> = withContext(Dispatchers.IO) {
        val request = authenticated(Request.Builder().url("${baseUrl()}/api/auth/identities")).get().build()
        val array = execute(request).optJSONArray("identities") ?: JSONArray()
        (0 until array.length()).mapNotNull { index ->
            array.optJSONObject(index)?.let { value ->
                LinkedLoginIdentity(
                    id = value.optString("id"),
                    provider = value.optString("provider"),
                    email = value.optionalString("email"),
                    displayName = value.optionalString("display_name"),
                )
            }
        }
    }

    suspend fun unlink(identityId: String) = withContext(Dispatchers.IO) {
        val request = authenticated(
            Request.Builder().url("${baseUrl()}/api/auth/identities/$identityId"),
        ).delete().build()
        execute(request)
        Unit
    }

    private suspend fun requestIdToken(clientId: String, nonce: String): String {
        val manager = CredentialManager.create(activity)
        suspend fun request(filterAuthorized: Boolean) = manager.getCredential(
            context = activity,
            request = GetCredentialRequest.Builder()
                .addCredentialOption(
                    GetGoogleIdOption.Builder()
                        .setFilterByAuthorizedAccounts(filterAuthorized)
                        .setServerClientId(clientId)
                        .setNonce(nonce)
                        .build(),
                )
                .build(),
        )
        val result = runCatching { request(true) }.getOrElse { request(false) }
        val credential = result.credential
        check(
            credential is CustomCredential &&
                credential.type == GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL,
        ) { "Google 没有返回可验证的 ID token" }
        return GoogleIdTokenCredential.createFrom(credential.data).idToken
    }

    private fun googleProvider(): JSONObject {
        val root = execute(Request.Builder().url("${baseUrl()}/api/auth/federation/providers").get().build())
        val providers = root.optJSONArray("providers") ?: JSONArray()
        return (0 until providers.length())
            .mapNotNull(providers::optJSONObject)
            .firstOrNull { it.optString("id") == "google" }
            ?: error("服务端没有公布 Google 登录能力")
    }

    private fun createChallenge(mode: String): JSONObject {
        val body = JSONObject()
            .put("mode", mode)
            .put("platform", "android")
            .put("request_id", "apk:challenge:${UUID.randomUUID()}")
            .put("client_instance_id", clientInstanceId())
        val builder = Request.Builder()
            .url("${baseUrl()}/api/auth/federation/google/challenges")
            .post(body.toString().toRequestBody(JSON))
        return execute(if (mode == "bind") authenticated(builder).build() else builder.build())
    }

    private fun complete(challengeId: String, idToken: String): JSONObject {
        val body = JSONObject()
            .put("challenge_id", challengeId)
            .put("id_token", idToken)
            .put("remember_device", true)
            .put("device_name", android.os.Build.MODEL)
            .put("apk_version", BuildConfig.VERSION_NAME)
            .put("request_id", "apk:complete:${UUID.randomUUID()}")
            .put("client_instance_id", clientInstanceId())
        val builder = Request.Builder()
            .url("${baseUrl()}/api/auth/federation/google/complete")
            .post(body.toString().toRequestBody(JSON))
        return execute(authenticated(builder).build())
    }

    private fun authenticated(builder: Request.Builder): Request.Builder =
        AuthManager.applyAuth(activity, builder)

    private fun baseUrl() = ElonApplication.activeServerUrl(activity).trimEnd('/')

    private fun clientInstanceId(): String {
        val preferences = activity.getSharedPreferences("elon_auth_security", Activity.MODE_PRIVATE)
        return preferences.getString("client_instance_id", null)?.takeIf(String::isNotBlank)
            ?: "apk:${UUID.randomUUID()}".also {
                preferences.edit().putString("client_instance_id", it).apply()
            }
    }

    private fun execute(request: Request): JSONObject = http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) {
            val message = runCatching { JSONObject(body).optString("error") }
                .getOrNull().orEmpty().ifBlank { "请求失败 (${response.code})" }
            error(message.take(500))
        }
        if (body.isBlank()) JSONObject() else JSONObject(body)
    }

    private fun JSONObject.optionalString(key: String): String? =
        if (!has(key) || isNull(key)) null else optString(key).trim().takeIf(String::isNotBlank)

    private companion object {
        val JSON = "application/json".toMediaType()
    }
}
