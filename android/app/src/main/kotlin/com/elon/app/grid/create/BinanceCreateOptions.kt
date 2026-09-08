package com.elon.app.grid.create

import java.math.BigDecimal

/** Optional fields follow the loaded manual create constructor (4818.ab7ba930). */
internal object BinanceCreateOptions {
    val defaults = mapOf("triggerPrice" to "", "triggerType" to "MARK_PRICE", "stopLower" to "", "stopUpper" to "",
        "stopType" to "MARK_PRICE", "trailingUp" to "false", "trailingDown" to "false",
        "trailingUpPrice" to "", "trailingDownPrice" to "", "autoAddMargin" to "false", "closeOnTpSl" to "true",
        "stopMode" to "PRICE", "stopProfit" to "", "stopLoss" to "")
    private val prices = setOf("triggerPrice", "stopLower", "stopUpper", "trailingUpPrice", "trailingDownPrice", "stopProfit", "stopLoss")
    fun normalize(values: Map<String, String>): Map<String, String> {
        val result = (defaults + values.filterKeys { it in defaults }).toMutableMap()
        for (key in prices) {
            val raw = result.getValue(key).trim()
            require(raw.isEmpty() || Regex("(0|[1-9][0-9]{0,19})(\\.[0-9]{1,20})?").matches(raw)) { "高级价格参数需填写普通十进制数" }
            result[key] = if (raw.isEmpty()) "" else BigDecimal(raw).also { require(it.signum() > 0) { "已填写的触发价格必须大于零" } }.stripTrailingZeros().toPlainString()
        }
        for (key in setOf("trailingUp", "trailingDown", "autoAddMargin", "closeOnTpSl")) require(result[key] in setOf("true", "false"))
        for (key in setOf("triggerType", "stopType")) require(result[key] in setOf("MARK_PRICE", "CONTRACT_PRICE"))
        require(result["stopMode"] in setOf("PRICE", "PNL", "ROI"))
        if(result["stopMode"] == "PRICE") require(result["stopProfit"].isNullOrEmpty() && result["stopLoss"].isNullOrEmpty()) { "价格模式下请清除金额或收益率阈值" }
        else require(result["stopLower"].isNullOrEmpty() && result["stopUpper"].isNullOrEmpty()) { "金额或收益率模式下请清除上下限终止价格" }
        require(result["trailingUpPrice"].isNullOrEmpty() || result["trailingUp"] == "true") { "请先启用追踪上涨，或清除追踪上涨限制价格" }
        require(result["trailingDownPrice"].isNullOrEmpty() || result["trailingDown"] == "true") { "请先启用追踪下跌，或清除追踪下跌限制价格" }
        if (result.getValue("stopLower").isNotEmpty() && result.getValue("stopUpper").isNotEmpty())
            require(BigDecimal(result.getValue("stopLower")) < BigDecimal(result.getValue("stopUpper"))) { "终止下限价格必须低于终止上限价格" }
        return result
    }
    fun payload(input: Map<String, String>): Map<String, Any> {
        if (input.keys.none { it in defaults }) return emptyMap()
        val p = normalize(input)
        val up = p["trailingUp"] == "true"; val down = p["trailingDown"] == "true"
        val result = linkedMapOf<String, Any>("autoAddMargin" to (p["autoAddMargin"] == "true"))
        if (p.getValue("triggerPrice").isNotEmpty()) {
            result["triggerPrice"] = p.getValue("triggerPrice"); result["triggerType"] = p.getValue("triggerType")
        }
        if (p.getValue("stopLower").isNotEmpty()) result["stopLowerLimit"] = p.getValue("stopLower")
        if (p.getValue("stopUpper").isNotEmpty()) result["stopUpperLimit"] = p.getValue("stopUpper")
        if ("stopLowerLimit" in result || "stopUpperLimit" in result) {
            result["stopTriggerType"] = p.getValue("stopType"); result["tpslCps"] = p["closeOnTpSl"] == "true"
        }
        if(p["stopMode"] != "PRICE" && (p.getValue("stopProfit").isNotEmpty() || p.getValue("stopLoss").isNotEmpty())) {
            val margin = BigDecimal(input.getValue("margin"))
            for ((key, field) in mapOf("stopProfit" to "stopTpPnl", "stopLoss" to "stopSlPnl")) {
                val raw = p.getValue(key).takeIf { it.isNotEmpty() } ?: continue
                val value = BigDecimal(raw)
                val ratio = p["stopMode"] == "ROI"
                val minimum = if(ratio) BigDecimal.ONE else margin.movePointLeft(2)
                require(value >= minimum) { "止盈止损阈值至少为投入保证金的 1%" }
                if(key == "stopLoss") require(value <= if(ratio) BigDecimal("100") else margin) { "止损阈值不能超过投入保证金或 100%" }
                val amount = if(ratio) margin.multiply(value).movePointLeft(2).setScale(2,
                    if(key == "stopProfit") java.math.RoundingMode.DOWN else java.math.RoundingMode.UP) else value
                require(amount.signum() > 0) { "换算后的止盈金额过小，请提高阈值" }
                result[field] = amount.stripTrailingZeros().toPlainString()
            }
            result["stopTriggerType"] = p.getValue("stopType"); result["tpslCps"] = p["closeOnTpSl"] == "true"
        }
        if (up || down) {
            result["trailingUp"] = up; result["trailingDown"] = down; result["orderCurrency"] = "QUOTE"
            if (up && p.getValue("trailingUpPrice").isNotEmpty()) result["trailingUpLimitPrice"] = p.getValue("trailingUpPrice")
            if (down && p.getValue("trailingDownPrice").isNotEmpty()) result["trailingDownLimitPrice"] = p.getValue("trailingDownPrice")
            result["trailingStopLowerLimit"] = p.getValue("stopLower").isNotEmpty() && up && !down && input["direction"] in setOf("LONG", "NEUTRAL")
            result["trailingStopUpperLimit"] = p.getValue("stopUpper").isNotEmpty() && !up && down && input["direction"] in setOf("SHORT", "NEUTRAL")
        }
        return result
    }
    fun summary(input: Map<String, String>): String {
        val fields = payload(input)
        fun price(key: String) = fields[key]?.toString() ?: "未设置"
        fun reference(key: String) = if (fields[key] == "CONTRACT_PRICE") "最新成交价" else "标记价格"
        return listOf("触发价：${price("triggerPrice")}（${reference("triggerType")}）",
            "终止下限 / 上限：${price("stopLowerLimit")} / ${price("stopUpperLimit")}（${reference("stopTriggerType")}）",
            "做多：下限止损、上限止盈；做空相反；中性按上下限终止。",
            "止盈 / 止损金额：${price("stopTpPnl")} / ${price("stopSlPnl")} USDT（收益率输入会换算为金额）",
            "止盈止损时平仓：${if (fields["tpslCps"] == true) "是" else if (fields["tpslCps"] == false) "否" else "未设置触发"}",
            "追踪上涨 / 下跌：${if (fields["trailingUp"] == true) "开启" else "关闭"} / ${if (fields["trailingDown"] == true) "开启" else "关闭"}",
            "追踪限制价：${price("trailingUpLimitPrice")} / ${price("trailingDownLimitPrice")}",
            "自动追加保证金：${if (fields["autoAddMargin"] == true) "开启" else "关闭"}").joinToString("\n")
    }
}
