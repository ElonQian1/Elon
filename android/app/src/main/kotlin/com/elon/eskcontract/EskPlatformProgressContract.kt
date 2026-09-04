package com.elon.eskcontract

import java.math.BigInteger

/** Pure, bounded progress protocol. Not Android identity, user consent, freshness polling or replay protection. */
object EskPlatformProgressContract {
    const val PROTOCOL = "yilong.esk.platform_android_progress.v1"
    const val ACTION = "com.elon.app.action.READ_ESK_PLATFORM_PROGRESS"
    const val REQUEST_WINDOW_MS = 120_000L
    const val DISPLAY_WINDOW_MS = 60_000L
    const val MAX_PAGE_COUNT = 20
    const val MAX_KEYS = 155
    const val MAX_KEY_LENGTH = 64
    const val MAX_VALUE_LENGTH = 128
    const val MAX_BYTES = 32_768
    val REQUEST_KEYS = setOf("protocol", "nonce", "cursor")
    val TOP_KEYS = setOf(
        "protocol", "nonce", "requested_cursor", "asset_id", "symbol", "decimals", "source", "chain_status",
        "simulated", "funds_moved", "verification_basis", "external_payment_verified", "total", "total_base_units",
        "reserved", "reserved_base_units", "available", "available_base_units", "snapshot_digest",
        "request_count", "open_count", "range_start", "range_end", "page_count", "has_more", "next_cursor",
        "observed_elapsed_ms", "expires_elapsed_ms", "service_spending", "quant_subscription",
        "sellback_settlement", "onchain_transfer", "chain_migration", "submit_request", "cancel_request",
    )
    private val fixedValues = mapOf(
        "protocol" to PROTOCOL, "asset_id" to "esk", "symbol" to "ESK", "decimals" to "6",
        "source" to "platform_recorded", "chain_status" to "not_deployed", "simulated" to "false",
        "funds_moved" to "false", "verification_basis" to "authenticated_operator_review",
        "external_payment_verified" to "false", "service_spending" to "false", "quant_subscription" to "false",
        "sellback_settlement" to "false", "onchain_transfer" to "false", "chain_migration" to "false",
        "submit_request" to "false", "cancel_request" to "false",
    )
    private val rowFields = listOf("id", "amount", "amount_base_units", "status", "created_at", "canceled_at")
    private val hex = Regex("[0-9a-f]{64}")
    private val cursorPattern = Regex("esbr1\\.[0-9a-f]{64}\\.eskpsr_[0-9a-f]{32}")
    private val integerPattern = Regex("0|[1-9][0-9]{0,18}")
    private val amountPattern = Regex("(0|[1-9][0-9]{0,12})\\.[0-9]{6}")
    private val maxUnits = BigInteger.valueOf(Long.MAX_VALUE)

    fun keysForCount(count: Int): Set<String> {
        if (count !in 0..MAX_PAGE_COUNT) return emptySet()
        return TOP_KEYS + (0 until count).flatMap { index -> rowFields.map { "request_${index}_$it" } }
    }

    private fun withinBudget(fields: Map<*, *>): Boolean {
        if (fields.size > MAX_KEYS) return false
        var bytes = 0
        for ((key, value) in fields) {
            if (key !is String || value !is String || key.length > MAX_KEY_LENGTH || value.length > MAX_VALUE_LENGTH) return false
            bytes += key.toByteArray(Charsets.UTF_8).size + value.toByteArray(Charsets.UTF_8).size
            if (bytes > MAX_BYTES) return false
        }
        return true
    }

    fun validCursor(value: String): Boolean = value.isEmpty() || (value.length == 110 && value.matches(cursorPattern))

    fun validRequest(fields: Map<String, String>): Boolean =
        withinBudget(fields) && fields.keys == REQUEST_KEYS && fields["protocol"] == PROTOCOL &&
            fields["nonce"]?.matches(hex) == true && fields["cursor"]?.let(::validCursor) == true

    fun validWindow(startedAt: Long, now: Long): Boolean =
        startedAt >= 0 && now >= startedAt && now - startedAt < REQUEST_WINDOW_MS

    fun integer(value: String): Long? = value.takeIf { it.matches(integerPattern) }?.toLongOrNull()

    fun units(value: String): BigInteger? {
        if (!value.matches(amountPattern)) return null
        return value.replace(".", "").toBigIntegerOrNull()?.takeIf { it <= maxUnits }
    }

    private fun amount(fields: Map<String, String>, name: String): Long? {
        val parsed = units(fields[name] ?: return null) ?: return null
        val base = integer(fields["${name}_base_units"] ?: return null) ?: return null
        return base.takeIf { parsed == BigInteger.valueOf(base) }
    }

    fun validSnapshot(fields: Map<String, String>, expectedNonce: String, expectedCursor: String,
        requestedAt: Long, now: Long): Boolean {
        // Budget/type guard must run before typed access, count parsing or dynamic key generation.
        if (!withinBudget(fields)) return false
        val pageCount = integer(fields["page_count"] ?: return false) ?: return false
        if (pageCount > MAX_PAGE_COUNT || fields.keys != keysForCount(pageCount.toInt())) return false
        if (!expectedNonce.matches(hex) || !validCursor(expectedCursor) || !validWindow(requestedAt, now)) return false
        if (fields["nonce"] != expectedNonce || fields["requested_cursor"] != expectedCursor ||
            fixedValues.any { (key, value) -> fields[key] != value }) return false
        val digest = fields["snapshot_digest"] ?: return false
        if (!digest.matches(hex)) return false
        val total = amount(fields, "total") ?: return false
        val reserved = amount(fields, "reserved") ?: return false
        val available = amount(fields, "available") ?: return false
        if (reserved > total || available != total - reserved) return false
        val count = integer(fields["request_count"] ?: return false) ?: return false
        val open = integer(fields["open_count"] ?: return false) ?: return false
        if (open > count || open > reserved || (open == 0L) != (reserved == 0L)) return false
        if (!validPage(fields, pageCount.toInt(), count, digest, expectedCursor)) return false
        if (!EskPlatformProgressRows.valid(fields, pageCount.toInt(), count, open, reserved, expectedCursor)) return false
        val observed = integer(fields["observed_elapsed_ms"] ?: return false) ?: return false
        val expires = integer(fields["expires_elapsed_ms"] ?: return false) ?: return false
        if (observed < requestedAt || observed > now || expires <= now) return false
        return expires - observed in 1L..DISPLAY_WINDOW_MS
    }

    private fun validPage(fields: Map<String, String>, size: Int, count: Long, digest: String, cursor: String): Boolean {
        val start = integer(fields["range_start"] ?: return false) ?: return false
        val end = integer(fields["range_end"] ?: return false) ?: return false
        val more = fields["has_more"] ?: return false
        val next = fields["next_cursor"] ?: return false
        if (count == 0L) return size == 0 && start == 0L && end == 0L && more == "false" && next.isEmpty() && cursor.isEmpty()
        if (size == 0 || start == 0L || end < start || end > count || end - start != size.toLong() - 1) return false
        if (cursor.isEmpty()) {
            if (start != 1L) return false
        } else if (start <= 1L || cursor.substring(6, 70) != digest) return false
        if (more != (end < count).toString()) return false
        return if (end < count) next == "esbr1.$digest.${fields["request_${size - 1}_id"]}" && next != cursor
            else next.isEmpty()
    }
}
