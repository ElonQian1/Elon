package com.elon.app.grid.access

import com.elon.app.privateaccess.*
import java.time.Instant
import java.util.Base64

/** This distinct native contract cannot upgrade an existing ESK grant. */
internal class GridAccessRequest private constructor(val state: String, val challenge: String) {
    fun approvalBody(): String = StrictJson.encode(mapOf("schema" to "yilong.asset_access.authorize.v1",
        "client_id" to "quant.android", "redirect_uri" to CALLBACK, "state" to state,
        "code_challenge" to challenge, "code_challenge_method" to "S256", "scopes" to listOf(SCOPE),
        "purpose" to PURPOSE, "expires_in" to 900, "explicit_consent" to true,
        "confirmation" to "授权量化只读我的币安网格"))

    fun validateResult(raw: String, now: Long): Boolean = runCatching {
        require(now >= 0)
        val value = StrictJson.parse(raw, 4096)
        value.exact("schema", "code", "state", "client_id", "redirect_uri", "code_expires_at", "grant_id", "expires_at", "scopes")
        require(value.text("schema") == "yilong.asset_access.authorization_code.v1")
        require(value.text("state") == state && value.text("client_id") == "quant.android" && value.text("redirect_uri") == CALLBACK)
        require(value.items("scopes") == listOf(SCOPE))
        require(Regex("aac_[0-9a-f]{64}").matches(value.text("code")))
        require(Regex("aag_[0-9a-f]{32}").matches(value.text("grant_id")))
        val expires = Instant.parse(value.text("expires_at")).toEpochMilli()
        val codeExpires = Instant.parse(value.text("code_expires_at")).toEpochMilli()
        // Small device/server skew is allowed only on future bounds, never on expiration.
        require(codeExpires > now && codeExpires - now <= 125_000 &&
            codeExpires <= expires && expires - now <= 905_000)
        true
    }.getOrDefault(false)
    override fun toString() = "GridAccessRequest(private)"
    companion object {
        const val INPUT = "grid_access_request"
        const val OUTPUT = "grid_access_approval"
        const val PURPOSE = "binance_grid_read"
        const val SCOPE = "grid.snapshot.read"
        const val CALLBACK = "com.elon.quant:/grid-access/callback"
        fun parse(raw: String?): GridAccessRequest? = runCatching {
            require(raw != null)
            val value = StrictJson.parse(raw, 1024)
            value.exact("schema", "purpose", "state", "code_challenge")
            require(value.text("schema") == "yilong.grid_access.android_request.v1" && value.text("purpose") == PURPOSE)
            val state = value.text("state"); val challenge = value.text("code_challenge")
            require(Regex("[A-Za-z0-9._~-]{32,128}").matches(state) && Regex("[A-Za-z0-9_-]{43}").matches(challenge))
            val bytes = Base64.getUrlDecoder().decode(challenge)
            require(bytes.size == 32 && Base64.getUrlEncoder().withoutPadding().encodeToString(bytes) == challenge)
            GridAccessRequest(state, challenge)
        }.getOrNull()
    }
}
