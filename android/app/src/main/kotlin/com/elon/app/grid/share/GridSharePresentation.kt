package com.elon.app.grid.share

import org.json.JSONObject
import java.math.BigDecimal
import java.math.RoundingMode

/** Display rounding never changes the exact decimal strings in the published snapshot. */
internal object GridSharePresentation {
    fun number(value: String?): BigDecimal? = value?.takeIf { it.length <= 100 && it.matches(Regex("[+-]?\\d+(?:\\.\\d+)?")) }?.toBigDecimalOrNull()
    fun format(value: String?, signed: Boolean = false): String {
        val amount = number(value) ?: return "未读取"
        val rounded = amount.setScale(8, RoundingMode.HALF_UP).stripTrailingZeros()
        if (amount.signum() != 0 && rounded.signum() == 0) return if (amount.signum() > 0) "<0.00000001" else "−<0.00000001"
        return (if (signed && amount.signum() > 0) "+" else "") + rounded.toPlainString()
    }
    fun token(symbol: String) = symbol.removeSuffix("USDT").removeSuffix("USDC").removeSuffix("BUSD").takeIf { it.matches(Regex(com.elon.app.grid.BinanceSymbols.BASE)) } ?: "?"
    fun raw(grid: JSONObject, key: String): String? {
        if (!grid.optBoolean("show_amounts") && key in GridShareModel.amounts) return null
        return grid.optJSONObject("fields")?.optString(key)?.takeIf { it.isNotBlank() }
    }
    fun metric(grid: JSONObject) = (if(raw(grid,"recordKind") == "HISTORY") listOf("totalPnl") else listOf("roi", "totalPnl", "profit")).firstOrNull { number(raw(grid, it)) != null }
    fun range(grid: JSONObject): Float? {
        val low = number(raw(grid, "lower")) ?: return null
        val high = number(raw(grid, "upper")) ?: return null
        val mark = number(raw(grid, "markPrice")) ?: return null
        if (high <= low) return null
        return ((mark - low).divide(high - low, 8, RoundingMode.HALF_UP)).toFloat().coerceIn(0f, 1f)
    }
    fun rangeLabel(grid: JSONObject): String {
        val low = number(raw(grid, "lower")) ?: return "价格区间"
        val high = number(raw(grid, "upper")) ?: return "价格区间"
        val mark = number(raw(grid, "markPrice")) ?: return "标记价未读取"
        return when { high <= low -> "价格区间"; mark < low -> "低于区间"; mark > high -> "高于区间"; else -> "区间内" }
    }
    fun sortRows(rows: List<Map<String, String?>>): List<Map<String, String?>> = rows.sortedWith { a, b ->
        val left = number(a["profit"]); val right = number(b["profit"])
        when { left == null && right != null -> 1; right == null && left != null -> -1
            left != null && right != null -> right.compareTo(left)
            else -> 0 }.takeIf { it != 0 } ?: (a["symbol"].orEmpty().compareTo(b["symbol"].orEmpty()))
    }
}
