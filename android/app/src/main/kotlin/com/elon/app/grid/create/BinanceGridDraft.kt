package com.elon.app.grid.create

import java.math.BigDecimal

/** Pure, bounded test draft. These limits are our input budget, not exchange trading rules. */
internal class BinanceGridDraft private constructor(val input: Map<String, String>) {
    val margin get() = input.getValue("margin")
    val symbol get() = input.getValue("symbol")
    fun payload(): Map<String, Any> = linkedMapOf<String, Any>(
        "symbol" to symbol, "direction" to input.getValue("direction"),
        "marginType" to input.getValue("marginType"), "gridType" to input.getValue("spacing"),
        "gridLowerLimit" to input.getValue("lower"), "gridUpperLimit" to input.getValue("upper"),
        "gridCount" to input.getValue("count").toInt(), "leverage" to input.getValue("leverage").toInt(),
        "gridInitialValue" to BigDecimal(margin).multiply(BigDecimal(input.getValue("leverage"))).stripTrailingZeros().toPlainString(),
        "cos" to true, "cps" to (input.getValue("closeOnStop") == "true"),
        "autoInitPos" to (input.getValue("autoInit") == "true"), "orderCurrency" to "BASE").apply {
            if (input["direction"] == "NEUTRAL") remove("autoInitPos")
            putAll(BinanceCreateOptions.payload(input))
        }

    fun summary(): String = listOf(
        "$symbol · ${when(input["direction"]) { "LONG" -> "做多"; "SHORT" -> "做空"; else -> "中性" }}",
        "保证金 $margin USDT · ${input["leverage"]} 倍杠杆",
        "创建名义金额 ${payload()["gridInitialValue"]} USDT",
        "${if (input["marginType"] == "ISOLATED") "逐仓" else "全仓（可能使用账户其他保证金）"}",
        "区间 ${input["lower"]} – ${input["upper"]} USDT · ${input["count"]} 格",
        if (input["spacing"] == "ARITH") "等差网格" else "等比网格",
        "创建时立即建仓：${if (input["direction"] == "NEUTRAL") "中性策略由币安处理" else if (input["autoInit"] == "true") "是" else "否"}",
        "终止时市价平仓：${if (input["closeOnStop"] == "true") "是" else "否，保留仓位"}",
        "终止时取消该合约未成交委托；已有委托请先在官网核对作用范围。",
        BinanceCreateOptions.summary(input),
        "投入金额不等于最大亏损保证。交易规则、费用和可用保证金最终由币安校验。"
    ).joinToString("\n")

    companion object {
        val KEYS = setOf("symbol", "direction", "spacing", "marginType", "lower", "upper", "margin", "leverage", "count", "autoInit", "closeOnStop")
        fun parse(values: Map<String, String>): BinanceGridDraft {
            require(values.keys.containsAll(KEYS) && values.keys.all { it in KEYS || it in BinanceCreateOptions.defaults }) { "参数字段不完整" }
            val input = values.mapValues { it.value.trim() }.toMutableMap()
            require(Regex("[A-Z0-9]{1,24}USDT").matches(input.getValue("symbol"))) { "请填写 U 本位合约，例如 NEARUSDT" }
            val names = mapOf("direction" to "方向", "spacing" to "网格间距", "marginType" to "保证金模式", "autoInit" to "是否立即建仓", "closeOnStop" to "终止时仓位处理")
            fun choice(key: String, allowed: Set<String>) { require(input[key] in allowed) { "请选择${names[key] ?: key}" } }
            choice("direction", setOf("LONG", "SHORT", "NEUTRAL")); choice("spacing", setOf("ARITH", "GEO"))
            choice("marginType", setOf("ISOLATED", "CROSSED"))
            choice("autoInit", setOf("true", "false")); choice("closeOnStop", setOf("true", "false"))
            fun decimal(key: String): BigDecimal {
                val value = input.getValue(key)
                require(Regex("(0|[1-9][0-9]{0,19})(\\.[0-9]{1,20})?").matches(value)) { "价格和保证金必须为普通十进制数" }
                return BigDecimal(value).also { require(it.signum() > 0) { "价格和保证金必须大于零" }; input[key] = it.stripTrailingZeros().toPlainString() }
            }
            require(decimal("lower") < decimal("upper")) { "上限必须大于下限" }
            require(decimal("margin") <= BigDecimal("2000")) { "本轮技术测试的保证金上限为 2000 USDT" }
            fun integer(key: String, range: IntRange) {
                require(Regex("[1-9][0-9]{0,4}").matches(input.getValue(key)) && input.getValue(key).toInt() in range) { "杠杆或格数超出本轮输入范围" }
            }
            integer("leverage", 1..125); integer("count", 2..10000)
            if (values.keys.any { it in BinanceCreateOptions.defaults }) input.putAll(BinanceCreateOptions.normalize(values))
            return BinanceGridDraft(input.toMap())
        }
    }
}
