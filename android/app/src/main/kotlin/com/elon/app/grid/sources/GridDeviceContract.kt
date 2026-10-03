package com.elon.app.grid.sources

import com.elon.app.grid.BinanceSymbols
import com.elon.app.grid.host.BinanceGridMetrics
import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal

/** A remote observation is never a local host grant or a trading target. */
internal class GridDeviceSource(val id: String, val platform: String, val device: String,
    val sequence: Long, val account: String?, val accountKind: String,
    val observed: Long, val freshUntil: Long, val status: String, val rows: List<Map<String, Any?>>) {
    val label get() = if (platform == "android") "APK 端" else "Win 端"
    fun fresh(now: Long) = status == "fresh" && now >= observed && now < freshUntil
    fun key(strategy: String) = "$id:$account:$accountKind:$strategy"
    override fun toString() = "GridDeviceSource(private)"
}

internal object GridDeviceContract {
    const val SCHEMA = "yilong.grid_device_snapshot.v1"
    private val hash = Regex("[0-9a-f]{64}")
    private val rowFields = setOf("id", "symbol", "status", "direction", "spacing", "lower", "upper",
        "count", "leverage", "profit", "created", "detail", "metrics")
    fun parse(raw: String, now: Long): List<GridDeviceSource> {
        val value = StrictJson.parse(raw, 1024 * 1024 + 16384, maxNodes = 500000)
        require(value.keys == setOf("schema", "sources") && value["schema"] == "yilong.grid_device_sources.v1")
        val sources = value["sources"] as? List<*> ?: error("SOURCES_INVALID")
        require(sources.size <= 16)
        return sources.map { item ->
            val entry = objectValue(item)
            require(entry.keys == setOf("source_id", "snapshot"))
            val id = entry["source_id"] as? String ?: error("SOURCE_ID_INVALID")
            require(hash.matches(id))
            val source = objectValue(entry["snapshot"])
            require(source.keys == setOf("schema", "platform", "device_id", "sequence", "account",
                "account_kind", "observed_at_ms", "fresh_until_ms", "status", "rows"))
            require(source["schema"] == SCHEMA && source["platform"] in setOf("android", "windows"))
            val sequence = number(source, "sequence")
            val observed = number(source, "observed_at_ms")
            val until = number(source, "fresh_until_ms")
            val account = source["account"] as? String
            require(source["account"] == null || account != null && hash.matches(account))
            require(source["account_kind"] in setOf("primary", "sub", "unknown"))
            require(sequence in 1..9_007_199_254_740_991L && observed in 1..(now + 30_000))
            require(until >= observed && until - observed <= 300_000)
            require(source["status"] in setOf("fresh", "unavailable"))
            val rows = source["rows"] as? List<*> ?: error("ROWS_INVALID")
            require(rows.size <= 500)
            val parsed = rows.map { row -> objectValue(row).also {
                require(it.keys == rowFields && it["detail"] is Boolean)
                require(it["id"] is String && Regex("[1-9][0-9]{0,19}").matches(it["id"] as String))
                require(it["symbol"] is String && BinanceSymbols.valid(it["symbol"] as String))
                require(it["status"] is String && Regex("[A-Z][A-Z0-9_]{0,63}").matches(it["status"] as String))
                require(it["direction"] == null || it["direction"] in setOf("LONG", "SHORT", "NEUTRAL"))
                require(it["spacing"] == null || it["spacing"] in setOf("ARITH", "GEO"))
                for (field in setOf("lower", "upper", "profit")) require(it[field] == null ||
                    it[field] is String && Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(it[field] as String))
                for (field in setOf("count", "leverage", "created")) require(it[field] == null ||
                    it[field] is String && Regex("[1-9][0-9]{0,15}").matches(it[field] as String))
                val lower = (it["lower"] as? String)?.let(::BigDecimal)
                val upper = (it["upper"] as? String)?.let(::BigDecimal)
                require(lower == null || lower.signum() > 0); require(upper == null || upper.signum() > 0)
                require(lower == null || upper == null || lower < upper)
                require((it["count"] as? String)?.toLong()?.let { it <= 100000 } != false)
                require((it["leverage"] as? String)?.length?.let { it <= 4 } != false)
                require((it["created"] as? String)?.toLong()?.let { it <= observed } != false)
                BinanceGridMetrics.decode(it["metrics"])
            } }
            require(parsed.map { it["id"] }.toSet().size == parsed.size)
            require(if (source["status"] == "unavailable") parsed.isEmpty() && account == null else account != null)
            val device = source["device_id"] as? String ?: error("DEVICE_INVALID")
            require(Regex("[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}").matches(device))
            GridDeviceSource(id, source["platform"] as String, device, sequence, account,
                source["account_kind"] as String, observed, until, source["status"] as String, parsed)
        }.also { require(it.map { source -> source.id }.toSet().size == it.size) }
    }
    private fun number(value: Map<String, Any?>, key: String): Long {
        val text = (value[key] as? StrictJson.Number)?.text ?: error("SOURCE_NUMBER_INVALID")
        require(Regex("0|[1-9][0-9]*").matches(text)); return text.toLong()
    }
    @Suppress("UNCHECKED_CAST")
    private fun objectValue(value: Any?): Map<String, Any?> = value as? Map<String, Any?> ?: error("SOURCE_OBJECT_INVALID")
}
