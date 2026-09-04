package com.elon.eskcontract

/** Public synthetic vectors only; shared byte-for-byte with the independent quant repository. */
internal object EskPlatformProgressFixtures {
    const val NONCE = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    const val DIGEST = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    const val REQUESTED_AT = 1_000L
    const val NOW = 3_000L

    fun id(number: Int): String = "eskpsr_" + number.toString(16).padStart(32, '0')
    fun cursor(number: Int): String = "esbr1.$DIGEST.${id(number)}"
    fun amount(units: Long): String = "${units / 1_000_000}.${(units % 1_000_000).toString().padStart(6, '0')}"

    fun row(number: Int, units: Long = 1, canceled: Boolean = false,
        created: String = "2026-09-04T12:00:00Z"): Map<String, String> = mapOf(
        "id" to id(number), "amount" to amount(units), "amount_base_units" to units.toString(),
        "status" to if (canceled) "canceled" else "submitted", "created_at" to created,
        "canceled_at" to if (canceled) "2026-09-04T12:00:01Z" else "",
    )

    fun page(
        rows: List<Map<String, String>> = listOf(row(2, 3), row(1, 7, true)),
        total: Long = 10, reserved: Long = 3, count: Long = rows.size.toLong(), open: Long = 1,
        start: Long = if (rows.isEmpty()) 0 else 1, requestedCursor: String = "",
    ): Map<String, String> {
        val end = if (rows.isEmpty()) 0 else start + rows.size - 1
        val more = end < count
        val result = linkedMapOf(
            "protocol" to "yilong.esk.platform_android_progress.v1", "nonce" to NONCE,
            "requested_cursor" to requestedCursor, "asset_id" to "esk", "symbol" to "ESK", "decimals" to "6",
            "source" to "platform_recorded", "chain_status" to "not_deployed", "simulated" to "false",
            "funds_moved" to "false", "verification_basis" to "authenticated_operator_review",
            "external_payment_verified" to "false", "total" to amount(total), "total_base_units" to "$total",
            "reserved" to amount(reserved), "reserved_base_units" to "$reserved",
            "available" to amount(total - reserved), "available_base_units" to "${total - reserved}",
            "snapshot_digest" to DIGEST, "request_count" to "$count", "open_count" to "$open",
            "range_start" to "$start", "range_end" to "$end", "page_count" to "${rows.size}",
            "has_more" to "$more", "next_cursor" to if (more) "esbr1.$DIGEST.${rows.last()["id"]}" else "",
            "observed_elapsed_ms" to "2000", "expires_elapsed_ms" to "62000",
            "service_spending" to "false", "quant_subscription" to "false", "sellback_settlement" to "false",
            "onchain_transfer" to "false", "chain_migration" to "false",
            "submit_request" to "false", "cancel_request" to "false",
        )
        rows.forEachIndexed { index, row -> row.forEach { (key, value) -> result["request_${index}_$key"] = value } }
        return result
    }

    fun accepts(fields: Map<String, String> = page(), cursor: String = "") =
        EskPlatformProgressContract.validSnapshot(fields, NONCE, cursor, REQUESTED_AT, NOW)
}
