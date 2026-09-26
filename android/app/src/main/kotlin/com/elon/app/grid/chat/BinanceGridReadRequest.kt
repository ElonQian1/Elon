package com.elon.app.grid.chat

internal data class BinanceGridReadRequest(
    val kind: String, val requestId: String, val start: Boolean = false,
    val strategyId: String? = null, val offset: Int = 0, val limit: Int = 25,
) {
    val identity get() = kind to strategyId
    companion object {
        fun parse(args: Map<String, Any?>): BinanceGridReadRequest {
            require(args.keys.all { it in setOf("kind", "request_id", "start", "strategy_id", "offset", "limit", "auth_token") })
            val kind = args["kind"] as? String; require(kind in setOf("list", "detail"))
            val request = args["request_id"] as? String ?: error("invalid_request")
            require(Regex("[A-Za-z0-9_-]{8,96}").matches(request))
            val start = if (args.containsKey("start")) args["start"] as? Boolean ?: error("invalid_start") else false
            val id = args["strategy_id"] as? String
            if (kind == "detail") require(id != null && Regex("[1-9][0-9]{0,19}").matches(id))
            else require(!args.containsKey("strategy_id"))
            fun integer(key: String, default: Int, range: IntRange): Int {
                val value = if (!args.containsKey(key)) default.toLong() else when (val v = args[key]) {
                    is Int -> v.toLong(); is Long -> v; else -> error("invalid_$key")
                }
                require(value in range.first.toLong()..range.last.toLong()); return value.toInt()
            }
            val offset = integer("offset", 0, 0..500); val limit = integer("limit", 25, 1..50)
            require(!start || offset == 0)
            require(kind == "list" || (!args.containsKey("offset") && !args.containsKey("limit")))
            return BinanceGridReadRequest(kind!!, request, start, id, offset, limit)
        }
    }
}
