package com.elon.app.grid.wallet

/** Structural diagnostics, never an authorization decision or credential. */
internal object BinanceWalletPermissionFacts {
    fun describe(recorded:Boolean,identityVerified:Boolean,matches:Boolean,active:Boolean):Map<String,Any> {
        val matched=recorded&&identityVerified&&matches
        val current=matched&&active
        val status=when {
            !recorded->"not_granted"
            !identityVerified->"identity_required"
            !matched->"account_mismatch"
            !current->"resume_required"
            else->"ready"
        }
        return mapOf("schema" to "yilong.binance_wallet_permission_facts.v1","status" to status,
            "consent_recorded" to recorded,"identity_verified" to identityVerified,
            "consent_matches_current_identity" to matched,"current_grant_active" to current)
    }
}
