package com.elon.eskcontract

import java.math.BigInteger

/** Android-only formal summary. Independent of the legacy Paper contract and all network APIs. */
object EskPlatformSnapshotContract {
    const val PROTOCOL = "yilong.esk.platform_android_snapshot.v1"
    const val ACTION = "com.elon.app.action.READ_ESK_PLATFORM_SNAPSHOT"
    const val REQUEST_WINDOW_MS = 120_000L
    const val DISPLAY_WINDOW_MS = 60_000L
    const val MAX_VALUE_LENGTH = 128
    val REQUEST_KEYS = setOf("protocol", "nonce")
    val KEYS = setOf(
        "protocol", "nonce", "asset_id", "symbol", "decimals", "source", "chain_status",
        "simulated", "funds_moved", "verification_basis", "external_payment_verified",
        "total", "total_base_units", "entry_count", "observed_elapsed_ms", "expires_elapsed_ms",
        "service_spending", "quant_subscription", "sellback_settlement", "onchain_transfer", "chain_migration",
    )
    private val fixedValues = mapOf(
        "asset_id" to "esk", "symbol" to "ESK", "decimals" to "6", "source" to "platform_recorded",
        "chain_status" to "not_deployed", "simulated" to "false", "funds_moved" to "false",
        "verification_basis" to "authenticated_operator_review", "external_payment_verified" to "false",
        "service_spending" to "false", "quant_subscription" to "false", "sellback_settlement" to "false",
        "onchain_transfer" to "false", "chain_migration" to "false",
    )
    private val noncePattern = Regex("[0-9a-f]{64}")
    private val integerPattern = Regex("0|[1-9][0-9]{0,18}")
    private val amountPattern = Regex("(0|[1-9][0-9]{0,12})\\.[0-9]{6}")
    private val maxUnits = BigInteger.valueOf(Long.MAX_VALUE)

    fun validRequest(fields: Map<String, String>): Boolean =
        fields.keys == REQUEST_KEYS && fields["protocol"] == PROTOCOL &&
            fields["nonce"]?.matches(noncePattern) == true

    fun validWindow(startedAt: Long, now: Long): Boolean =
        startedAt >= 0 && now >= startedAt && now - startedAt < REQUEST_WINDOW_MS

    fun integer(value: String): Long? =
        value.takeIf { it.matches(integerPattern) }?.toLongOrNull()

    fun units(value: String): BigInteger? {
        if (!value.matches(amountPattern)) return null
        return value.replace(".", "").toBigIntegerOrNull()?.takeIf { it <= maxUnits }
    }

    fun validSnapshot(
        fields: Map<String, String>,
        expectedNonce: String,
        requestedAt: Long,
        now: Long,
    ): Boolean {
        if (fields.keys != KEYS || fields.values.any { it.length > MAX_VALUE_LENGTH }) return false
        if (!validWindow(requestedAt, now) || !expectedNonce.matches(noncePattern)) return false
        if (fields["protocol"] != PROTOCOL || fields["nonce"] != expectedNonce ||
            fixedValues.any { (key, value) -> fields[key] != value }
        ) return false
        val total = units(fields["total"] ?: return false) ?: return false
        val baseUnits = integer(fields["total_base_units"] ?: return false) ?: return false
        val count = integer(fields["entry_count"] ?: return false) ?: return false
        if (total != BigInteger.valueOf(baseUnits) || (count == 0L) != (baseUnits == 0L) || count > baseUnits) return false
        val observed = integer(fields["observed_elapsed_ms"] ?: return false) ?: return false
        val expires = integer(fields["expires_elapsed_ms"] ?: return false) ?: return false
        // Ordering and nonnegative parsing precede subtraction, so neither difference can overflow.
        if (observed < requestedAt || observed > now || expires <= now) return false
        return expires - observed in 1L..DISPLAY_WINDOW_MS
    }
}
