package com.elon.app.esk.platform.access

import org.json.JSONArray
import org.json.JSONObject

/** The main app chooses audience, scopes and lifetime. The caller supplies only its PKCE binding. */
internal class AssetAccessRequest private constructor(val state: String, val challenge: String) {
    fun approvalBody(): String = JSONObject().put("schema", "yilong.asset_access.authorize.v1")
        .put("client_id", "quant.android").put("redirect_uri", CALLBACK).put("state", state)
        .put("code_challenge", challenge).put("code_challenge_method", "S256")
        .put("scopes", JSONArray(SCOPES)).put("expires_in", 900).put("explicit_consent", true)
        .put("confirmation", "授权量化只读我的资产").toString()

    fun validateResult(raw: String, nowMillis: Long): Boolean = runCatching {
        require(raw.length <= 4096)
        val value = JSONObject(raw)
        require(value.keys().asSequence().toSet() == setOf("schema", "code", "state", "client_id",
            "redirect_uri", "code_expires_at", "grant_id", "expires_at", "scopes"))
        require(value.getString("schema") == "yilong.asset_access.authorization_code.v1")
        require(value.getString("state") == state && value.getString("client_id") == "quant.android")
        require(value.getString("redirect_uri") == CALLBACK)
        require(Regex("aac_[0-9a-f]{64}").matches(value.getString("code")))
        require(Regex("aag_[0-9a-f]{32}").matches(value.getString("grant_id")))
        val scopes = value.getJSONArray("scopes")
        require(scopes.length() == SCOPES.size && (0 until scopes.length()).map(scopes::getString).toSet() == SCOPES.toSet())
        val expiry = java.time.Instant.parse(value.getString("expires_at")).toEpochMilli()
        val codeExpiry = java.time.Instant.parse(value.getString("code_expires_at")).toEpochMilli()
        require(codeExpiry > nowMillis && codeExpiry <= nowMillis + 120_000 && codeExpiry <= expiry && expiry <= nowMillis + 900_000)
        true
    }.getOrDefault(false)

    override fun toString() = "AssetAccessRequest(redacted)"

    companion object {
        const val INPUT = "asset_access_request"
        const val OUTPUT = "asset_access_approval"
        const val CALLBACK = "com.elon.quant:/asset-access/callback"
        val SCOPES = listOf("esk.summary.read", "esk.progress.read")

        fun parse(raw: String?): AssetAccessRequest? = runCatching {
            require(raw != null && raw.length <= 1024)
            val value = JSONObject(raw)
            require(value.keys().asSequence().toSet() == setOf("schema", "state", "code_challenge"))
            require(value.getString("schema") == "yilong.asset_access.android_request.v1")
            val state = value.getString("state")
            val challenge = value.getString("code_challenge")
            require(Regex("[A-Za-z0-9._~-]{32,128}").matches(state))
            require(Regex("[A-Za-z0-9_-]{43}").matches(challenge))
            val decoded = java.util.Base64.getUrlDecoder().decode(challenge)
            require(decoded.size == 32 && java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(decoded) == challenge)
            AssetAccessRequest(state, challenge)
        }.getOrNull()
    }
}
