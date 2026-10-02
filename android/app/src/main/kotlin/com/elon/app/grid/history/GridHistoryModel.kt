package com.elon.app.grid.history

import com.elon.app.grid.host.BinanceReportFields

internal object GridHistoryModel {
    const val NOTICE = "历史记录仅描述结束时已读取的事实。网格利润不等于最终总盈亏；手续费是否已含、最终平仓、收益率分母、追加撤出、调整明细及收益曲线未读取时均为未知。不能把此记录当成当前持仓，也不能据此推算最大回撤。"
    fun project(row: Map<String, Any?>, observed: Long): Map<String, String?> {
        val decoded = BinanceReportFields.decode("history", row)
        check(decoded["status"] !in setOf("NEW", "WORKING", "RUNNING")) { "历史列表混入运行中记录" }
        val fields = decoded.mapValues { (key, value) -> (value as? String)?.takeUnless { key in setOf("end", "created") && it == "0" } }
        val end = fields["end"]?.toLongOrNull()
        val start = fields["created"]?.toLongOrNull()
        check(start == null || start > 0 && start <= observed + 5000) { "历史开始时间无效" }
        check(end == null || end > 0 && end <= observed + 5000 && (start == null || end >= start)) { "历史时间无效" }
        return fields + mapOf("recordKind" to "HISTORY", "settlement" to "UNKNOWN", "positionState" to "UNKNOWN",
            "endReason" to "UNKNOWN", "feeBasis" to "UNKNOWN", "roiBasis" to "UNKNOWN")
    }
}
