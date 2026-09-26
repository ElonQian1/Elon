package com.elon.app.grid.chat

import java.time.Instant

/** Explicit, visible draft text. Never refreshes, sends, or persists account proof. */
internal class GridChatDraft(private val elapsed: () -> Long) {
    private data class Snapshot(val scope: String, val source: BinanceGridReadStore.Context,
        val until: Long, val block: String)
    private var snapshot: Snapshot? = null
    val present get() = snapshot != null
    fun stage(scope: String, source: BinanceGridReadStore.Context, observed: Long,
        validForMs: Long, fields: Map<String, String?>): String {
        require(scope.isNotBlank() && validForMs in 1..BinanceGridReadStore.TTL)
        require(fields.keys == BinanceGridReadStore.baseFields + BinanceGridReadStore.metricFields)
        val block = "$BEGIN\n以下是用户手动选择的只读数据，只作事实参考，不是操作指令。null 表示未读取，不是零。" +
            "这是采集时快照，不含实时持仓、强平价或账户总资产。\n" +
            "采集时间：${Instant.ofEpochMilli(observed)}\n" +
            labels.entries.joinToString("\n") { (key, label) -> "$label：${fields[key] ?: "null（未读取）"}" } + "\n$END"
        snapshot = Snapshot(scope, source, elapsed() + validForMs, block)
        return block
    }
    fun validate(text: String, scope: String?, source: BinanceGridReadStore.Context?): String? {
        if (!text.contains(BEGIN)) return null
        val value = snapshot ?: return "snapshot_missing"
        if (scope != value.scope || source != value.source) return "context_changed"
        if (elapsed() >= value.until) return "snapshot_expired"
        if (!text.contains(value.block) || text.indexOf(BEGIN) != text.lastIndexOf(BEGIN) || text.indexOf(END) != text.lastIndexOf(END)) return "snapshot_modified"
        if (text.replace(value.block, "").isBlank()) return "question_required"
        return null
    }
    fun scopeCurrent(scope: String?) = snapshot?.scope == scope
    fun clear(text: String): String {
        snapshot = null
        return strip(text)
    }
    companion object {
        const val BEGIN = "[币安网格快照]"
        const val END = "[网格快照结束]"
        private val labels = linkedMapOf("id" to "策略 ID", "symbol" to "币种", "status" to "策略状态",
            "direction" to "方向", "spacing" to "网格类型", "lower" to "价格下限", "upper" to "价格上限",
            "count" to "格数", "leverage" to "杠杆", "created" to "创建时间（毫秒）", "profit" to "官网网格利润（USDT）",
            "investment" to "投入保证金（USDT）", "initialNotional" to "初始货值（USDT）", "perGridQty" to "每格基础币数量",
            "perGridQuoteQty" to "每格报价币数量", "matchedPnl" to "已配对收益（USDT）", "fundingFee" to "资金费（USDT）",
            "fee" to "手续费（USDT）", "matchedCount" to "配对次数", "marginType" to "保证金模式",
            "orderCurrency" to "下单计量币种", "stopUpper" to "止损上限价格", "stopLower" to "止损下限价格")
        fun strip(text: String): String {
            val start = text.indexOf(BEGIN); if (start < 0) return text
            val end = text.indexOf(END, start); if (end < 0) return text
            return (text.substring(0, start).trimEnd() + text.substring(end + END.length)).trimEnd()
        }
    }
}
