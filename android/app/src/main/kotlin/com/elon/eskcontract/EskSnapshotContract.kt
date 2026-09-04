package com.elon.eskcontract

import java.math.BigInteger

/** Versioned Android-only wire contract. Keep byte-identical in both repositories. */
object EskSnapshotContract {
    const val PROTOCOL = "yilong.esk.android_snapshot.v1"
    const val ACTION = "com.elon.app.action.READ_ESK_SNAPSHOT"
    const val REQUEST_WINDOW_MS = 120_000L
    const val DISPLAY_WINDOW_MS = 60_000L
    val REQUEST_KEYS = setOf("protocol", "nonce")
    val KEYS = setOf(
        "protocol", "nonce", "asset_id", "symbol", "mode", "issuance_mode", "chain_status",
        "simulated", "funds_moved", "total", "available", "reserved_for_sellback",
        "reserved_for_quant", "reserved_total", "revision", "observed_elapsed_ms", "expires_elapsed_ms",
    )
    private val noncePattern = Regex("^[0-9a-f]{64}$")
    private val integerPattern = Regex("^(0|[1-9][0-9]{0,18})$")
    private val amountPattern = Regex("^(0|[1-9][0-9]{0,12})\\.[0-9]{6}$")
    private val maxUnits = BigInteger.valueOf(Long.MAX_VALUE)

    fun validRequest(fields: Map<String, String>): Boolean =
        fields.keys == REQUEST_KEYS && fields["protocol"] == PROTOCOL &&
            fields["nonce"]?.matches(noncePattern) == true

    fun validWindow(startedAt: Long, now: Long): Boolean =
        startedAt >= 0 && now >= startedAt && now - startedAt < REQUEST_WINDOW_MS

    fun units(value: String): BigInteger? {
        if (!value.matches(amountPattern)) return null
        return value.replace(".", "").toBigIntegerOrNull()?.takeIf { it <= maxUnits }
    }

    fun integer(value: String): Long? =
        value.takeIf { it.matches(integerPattern) }?.toLongOrNull()

    fun validBalances(fields: Map<String, String>): Boolean {
        val total = units(fields["total"] ?: return false) ?: return false
        val available = units(fields["available"] ?: return false) ?: return false
        val sellback = units(fields["reserved_for_sellback"] ?: return false) ?: return false
        val quant = units(fields["reserved_for_quant"] ?: return false) ?: return false
        val reserved = units(fields["reserved_total"] ?: return false) ?: return false
        return total == available + reserved && reserved == sellback + quant
    }

    fun validSnapshot(
        fields: Map<String, String>,
        expectedNonce: String,
        requestedAt: Long,
        now: Long,
    ): Boolean {
        if (fields.keys != KEYS || fields.values.any { it.length > 128 }) return false
        if (!validWindow(requestedAt, now) || !expectedNonce.matches(noncePattern)) return false
        if (fields["protocol"] != PROTOCOL || fields["nonce"] != expectedNonce ||
            fields["asset_id"] != "esk" || fields["symbol"] != "ESK" ||
            fields["mode"] !in setOf("paper", "disabled") ||
            fields["issuance_mode"] != "paper_recorded" || fields["chain_status"] != "not_deployed" ||
            fields["simulated"] != "true" || fields["funds_moved"] != "false"
        ) return false
        if (integer(fields["revision"] ?: return false) == null || !validBalances(fields)) return false
        val observed = integer(fields["observed_elapsed_ms"] ?: return false) ?: return false
        val expires = integer(fields["expires_elapsed_ms"] ?: return false) ?: return false
        return observed >= requestedAt && observed <= now && expires > now &&
            expires >= observed && expires - observed == DISPLAY_WINDOW_MS
    }
}
