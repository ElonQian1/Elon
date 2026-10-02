package com.elon.app.grid.share

import org.json.JSONArray
import org.json.JSONObject

/** Public projection. Exchange identity and strategy IDs stay in the local update binding. */
internal object GridShareModel {
    const val PREFIX = "【一龙AI对话】\n"
    const val SCHEMA = "yilong.grid_share.v1"
    val amounts = setOf("investment", "initialNotional", "perGridQty", "perGridQuoteQty", "profit", "matchedPnl",
        "fundingFee", "fee", "positionQty", "positionNotional", "totalPnl", "unrealizedPnl")
    val labels = linkedMapOf("symbol" to "合约", "direction" to "方向", "status" to "采集时状态", "leverage" to "杠杆",
        "lower" to "区间下限", "upper" to "区间上限", "count" to "网格数量", "spacing" to "间距",
        "markPrice" to "标记价格", "roi" to "策略总收益率 (%)", "entryPrice" to "持仓均价", "liquidationPrice" to "预估强平价",
        "investment" to "投入保证金 (USDT)", "initialNotional" to "初始货值 (USDT)", "perGridQty" to "每格基础币数量",
        "perGridQuoteQty" to "每格报价币数量", "profit" to "网格利润 (USDT)", "matchedPnl" to "已配对收益 (USDT)",
        "fundingFee" to "资金费 (USDT)", "fee" to "手续费 (USDT)", "positionQty" to "实际持仓数量",
        "positionNotional" to "持仓货值 (USDT)", "totalPnl" to "策略总盈亏 (USDT)", "unrealizedPnl" to "未实现盈亏 (USDT)",
        "matchedCount" to "配对次数", "marginType" to "保证金模式", "orderCurrency" to "下单计量", "stopUpper" to "止损上限", "stopLower" to "止损下限")
    val sections = linkedMapOf("持仓" to listOf("positionQty", "positionNotional", "entryPrice", "markPrice", "liquidationPrice", "marginType"),
        "收益" to listOf("roi", "totalPnl", "profit", "matchedPnl", "unrealizedPnl", "fundingFee", "fee", "matchedCount"),
        "参数" to listOf("direction", "leverage", "lower", "upper", "count", "spacing", "investment", "initialNotional", "perGridQty", "perGridQuoteQty", "orderCurrency", "stopUpper", "stopLower"))

    fun project(row: Map<String, String?>, observed: Long, showAmounts: Boolean = true, note: String = "", previous: String? = null): JSONObject {
        require(row["symbol"] != null)
        val fields = JSONObject()
        labels.keys.filter { showAmounts || it !in amounts }.forEach { key -> row[key]?.let { fields.put(key, it) } }
        return JSONObject().put("schema", SCHEMA).put("observed_at_ms", observed).put("show_amounts", showAmounts).put("fields", fields)
            .apply { if (note.isNotBlank()) put("note", note.trim().take(200)); if (previous != null) put("previous_snapshot_id", previous) }
    }
    fun value(grid: JSONObject, key: String): String {
        if (!grid.optBoolean("show_amounts") && key in amounts) return "未公开"
        val fields = grid.optJSONObject("fields") ?: return "未读取"
        if (!fields.has(key) || fields.isNull(key)) return "未读取"
        return when (val value = fields.optString(key)) {
            "LONG" -> "做多"; "SHORT" -> "做空"; "NEUTRAL" -> "中性"; "ARITH" -> "等差"; "GEO" -> "等比"
            "CROSSED" -> "全仓"; "ISOLATED" -> "逐仓"; "BASE" -> "基础币"; "QUOTE" -> "报价币"
            "WORKING", "NEW" -> "运行中"; else -> value
        }
    }
    fun document(grid: JSONObject): JSONObject {
        val fields = grid.getJSONObject("fields")
        fun get(key: String) = fields.optString(key).ifBlank { "未读取" }
        val direction = value(grid, "direction").let { if (it == "未读取") "方向未读取" else it }
        return JSONObject().put("schema", "elon.ai_conversation_share.v1").put("provider", "binance")
            .put("title", "${get("symbol")} 网格快照")
            .put("summary", "$direction · ${get("leverage")}× · ${get("lower")}–${get("upper")} · ${get("count")} 格 · 历史快照")
            .put("messages", JSONArray()).put("grid", grid)
    }
    fun quoteSummary(text: String): String? = card(text)?.getJSONObject("grid")?.let { grid ->
        val symbol = value(grid, "symbol").take(40)
        val direction = value(grid, "direction").take(16)
        val leverage = grid.optJSONObject("fields")?.optString("leverage")?.takeIf { it.matches(Regex("\\d{1,3}(?:\\.\\d{1,2})?")) }
        "[网格快照] $symbol · $direction" + (leverage?.let { " $it×" } ?: "")
    }
    fun card(text: String): JSONObject? = runCatching {
        if (!text.startsWith(PREFIX) || text.length > 8000) return null
        JSONObject(text.removePrefix(PREFIX)).takeIf { it.optString("schema") == "elon.ai_conversation_share.v1"
            && it.optString("provider") == "binance" && it.optJSONObject("grid")?.optString("schema") == SCHEMA
            && !it.optJSONObject("grid")?.optJSONObject("fields")?.optString("symbol").isNullOrBlank()
            && it.optString("snapshot_id").matches(Regex("ai_snapshot_[A-Za-z0-9_-]+"))
            && it.optString("group_id").matches(Regex("[A-Za-z0-9_-]{1,160}")) }
    }.getOrNull()
}
