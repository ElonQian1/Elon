package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import java.security.MessageDigest
import java.security.SecureRandom

/** Thread confinement is supplied by the host's main looper. No website credentials here. */
internal class BinanceHostState(private val elapsed: () -> Long, private val epoch: () -> Long) {
    var account: String? = null; private set
    var generation = 0L; private set
    var observed = 0L; private set
    var observedElapsed = 0L; private set
    var ready = false; private set
    private var rows = linkedMapOf<String, Map<String, Any?>>()
    private val grants = mutableMapOf<String, Long>()
    val count get() = rows.size

    fun unavailable() { ready = false; rows.clear(); grants.clear(); generation++ }
    fun accept(raw: String) {
        val event = StrictJson.parse(raw)
        when (event["kind"]) {
            "list" -> {
                val list = event["rows"] as? List<*> ?: error("LIST_INVALID")
                require(list.size <= 500)
                val decoded = list.map(::decode)
                val identities = decoded.map { it["account"] as? String ?: error("ACCOUNT_UNVERIFIED") }.toSet()
                // An empty observed page does not independently identify its Binance account.
                require(identities.size == 1 && decoded.map { it["id"] }.toSet().size == decoded.size)
                val nextAccount = digest(identities.single())
                if (account != null && account != nextAccount) grants.clear()
                account = nextAccount
                rows = linkedMapOf<String, Map<String, Any?>>().apply {
                    decoded.forEach { put(it["id"] as String, it.filterKeys { key -> key != "account" } + ("detail" to false)) }
                }
                observed = epoch(); observedElapsed = elapsed(); generation++; ready = true
            }
            "detail" -> {
                require(fresh())
                val row = decode(event["row"])
                val old = rows[row["id"]] ?: error("UNKNOWN_GRID")
                require(old["symbol"] == row["symbol"])
                val identity = row["account"] as? String
                require(identity == null || digest(identity) == account)
                rows[row["id"] as String] = row.filterKeys { it != "account" } + ("detail" to true)
                generation++
            }
            else -> unavailable()
        }
    }
    fun fresh() = ready && elapsed() - observedElapsed in 0 until 300_000
    fun grant(): String {
        require(fresh() && account != null)
        grants.entries.removeAll { it.value <= elapsed() }
        require(grants.size < 8)
        val token = ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
        grants[token] = Math.addExact(elapsed(), 900_000)
        return token
    }
    fun remaining(token: String) = ((grants[token] ?: 0L) - elapsed()).coerceAtLeast(0)
    fun authorized(token: String) = Regex("[0-9a-f]{64}").matches(token) && remaining(token) > 0
    fun revoke(token: String) { grants.remove(token) }
    fun contains(id: String) = fresh() && rows.containsKey(id)
    fun reply(token: String): String {
        require(authorized(token))
        return StrictJson.encode(mapOf("schema" to "yilong.binance_host_read.v1", "source" to "android_webview",
            "remaining_ms" to remaining(token), "generation" to generation, "observed_at_ms" to observed,
            "status" to if (fresh()) "fresh" else "stale", "coverage" to "observed_response_only",
            "rows" to if (fresh()) rows.values.toList() else emptyList<Any>()))
    }
    private fun decode(value: Any?): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST") val row = value as? Map<String, Any?> ?: error("ROW_INVALID")
        require(row.keys == setOf("id", "account", "symbol", "status", "direction", "spacing", "lower", "upper", "count", "leverage", "profit", "created"))
        fun text(key: String, pattern: String, nullable: Boolean = false): String? {
            val field = row[key]
            if (nullable && field == null) return null
            require(field is String && Regex(pattern).matches(field)); return field
        }
        text("id", "[0-9]{1,20}"); text("account", "[0-9]{1,20}", true)
        text("symbol", "[A-Z0-9]{1,24}USDT"); text("status", "[A-Z][A-Z0-9_]{0,63}")
        text("direction", "LONG|SHORT|NEUTRAL", true); text("spacing", "ARITH|GEO", true)
        val decimal = "(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?"
        val lower = text("lower", decimal, true)?.let(::BigDecimal)
        val upper = text("upper", decimal, true)?.let(::BigDecimal)
        require(lower == null || lower.signum() > 0); require(upper == null || upper.signum() > 0)
        require(lower == null || upper == null || lower < upper)
        text("profit", "-?$decimal", true)
        text("count", "[1-9][0-9]{0,5}", true)?.let { require(it.toLong() <= 100_000) }
        text("leverage", "[1-9][0-9]{0,3}", true)
        text("created", "[1-9][0-9]{0,15}", true)?.let { require(it.toLong() <= epoch()) }
        return row
    }
    companion object {
        fun digest(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray())
            .joinToString("") { "%02x".format(it) }
    }
}
