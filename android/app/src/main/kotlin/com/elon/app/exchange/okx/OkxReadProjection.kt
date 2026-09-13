package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import java.util.Locale

/** Android projection of the existing grid-center.v1 OKX contract; money not verified stays unknown. */
internal object OkxReadProjection {
    private fun known(value: Any?) = mapOf("value" to value, "missing_reason" to null)
    private fun unknown(reason: String = "not_reported") = mapOf("value" to null, "missing_reason" to reason)
    private fun decimal(value: String, positive: Boolean = false): String {
        require(value.length in 1..64 && Regex("-?(0|[1-9][0-9]*)(\\.[0-9]+)?").matches(value))
        val parsed = BigDecimal(value).stripTrailingZeros()
        require(parsed.scale() <= 28 && parsed.precision() <= 29 && (!positive || parsed.signum() > 0))
        return parsed.toPlainString()
    }
    private fun text(row: Map<String, Any?>, key: String) = row[key] as? String ?: error("INVALID_FIELD")
    private fun optional(row: Map<String, Any?>, key: String): String? {
        if (!row.containsKey(key) || row[key] == null) return null
        return (row[key] as? String ?: error("INVALID_FIELD")).also { require(it.length <= 40 && it.all { c -> c.code < 128 }) }
    }
    private fun enumeration(value: String?, mapping: Map<String, String>): Map<String, Any?> = when {
        value.isNullOrEmpty() -> unknown()
        value in mapping -> known(mapping.getValue(value))
        else -> unknown("not_verified")
    }
    fun bot(row: Map<String, Any?>, generation: Long, now: Long, detail: Boolean): Map<String, Any?> {
        val id = OkxReadProtocol.id(text(row, "algoId"))
        require(row["algoOrdType"] == "contract_grid")
        val instrument = text(row, "instId")
        require(Regex("[A-Z0-9]{1,20}-USDT-SWAP").matches(instrument))
        for ((key, value) in mapOf("instType" to "SWAP", "ctType" to "linear", "settleCcy" to "USDT"))
            if (row.containsKey(key) && row[key] != null) require(row[key] == value)
        val state = text(row, "state").also { require(Regex("[a-z_]{1,40}").matches(it)) }
        val lower = decimal(text(row, "minPx"), true)
        val upper = decimal(text(row, "maxPx"), true)
        require(BigDecimal(lower) < BigDecimal(upper))
        val count = text(row, "gridNum").also { require(Regex("[1-9][0-9]{0,5}").matches(it)) }.toLong()
        require(count <= 100_000)
        val created = text(row, "cTime").also { require(Regex("[1-9][0-9]{0,18}").matches(it)) }.toLong()
        require(created in 1..now)
        for (key in listOf("pnl", "totalPnl")) if (row[key] != null) {
            val value = text(row, key); if (value.isNotEmpty()) decimal(value)
        }
        val lever = optional(row, "lever")?.takeIf(String::isNotEmpty)?.let { known(decimal(it, true)) } ?: unknown()
        val bot = mapOf("schema_version" to "grid-center.v1", "id" to id,
            "strategy_host" to "okx_hosted", "execution_venue" to "okx", "environment" to "live",
            "instrument" to mapOf("id" to instrument, "base_asset" to instrument.removeSuffix("-USDT-SWAP"),
                "quote_asset" to "USDT", "settlement_asset" to "USDT", "market_kind" to "linear_perpetual",
                "quantity_unit" to "contracts", "contract_multiplier" to unknown()),
            "created_at_ms" to known(created), "status" to if (state == "running") "running" else "unknown",
            "status_reason" to if (state == "running") null else "not_verified",
            "parameters" to mapOf("direction" to enumeration(optional(row, "direction"), mapOf("long" to "long", "short" to "short", "neutral" to "neutral")),
                "spacing" to enumeration(optional(row, "runType"), mapOf("1" to "arithmetic", "2" to "geometric")),
                "lower_price" to known(lower), "upper_price" to known(upper), "grid_count" to known(count),
                "quantity_per_grid" to unknown(), "leverage" to lever),
            "metrics" to mapOf("currency" to "USDT", "exchange_reported_profit" to unknown("not_verified"),
                "realized_gross_pnl" to unknown(), "unrealized_gross_pnl" to unknown(),
                "reconciled_net_pnl" to unknown("not_reconciled"), "fees" to unknown(), "funding_fees" to unknown()),
            "provenance" to mapOf("observed_at_ms" to now, "received_at_ms" to now, "generation" to generation,
                "simulated" to false, "source_kind" to "exchange_reported"))
        return mapOf("bot" to bot, "provider_status" to state.uppercase(Locale.ROOT), "detail_available" to detail)
    }
    fun encode(account: OkxAccount, generation: Long, revision: Long, now: Long, bots: List<Map<String, Any?>>, detailId: String?): String {
        val json = StrictJson.encode(mapOf("schema" to OkxReadProtocol.SCHEMA, "environment" to "live",
            "account" to account.reference, "account_kind" to account.kind, "generation" to generation,
            "revision" to revision, "observed_at_ms" to now, "fresh_until_ms" to now + 60_000,
            "detail_id" to detailId, "complete" to (detailId == null), "bots" to bots))
        // Bundle strings travel as UTF-16; leave ample space below Android's shared Binder transaction cap.
        if (json.toByteArray(Charsets.UTF_8).size > 196_608) okxFail(OkxReadFailure.RESPONSE_LIMIT)
        return json
    }
}
