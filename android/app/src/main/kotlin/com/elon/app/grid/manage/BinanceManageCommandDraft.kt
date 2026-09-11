package com.elon.app.grid.manage

import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson

/** One typed intention. Exchange validation remains in the existing management domain and adapter. */
internal data class BinanceManageCommandDraft(val id: String, val action: String, val cps: Boolean,
    val amount: String, val range: BinanceRangeDraft?,val protection:BinanceProtectionDraft?=null) {
    companion object {
        fun parse(raw: String,version:Int=2): BinanceManageCommandDraft {
            val values = StrictJson.parse(raw, 4096)
            require(values.values.all { it is String }) { "参数格式不完整" }
            fun value(key: String) = values[key] as? String ?: error("缺少管理参数")
            fun flag(key: String) = value(key).let { require(it in setOf("true", "false")); it == "true" }
            val action = value("action")
            val extra = when(action) {
                "settings", "close" -> setOf("cps")
                "investment" -> setOf("amount")
                "range" -> setOf("lower", "upper", "count", "close_positions", "amount")
                "protection" -> BinanceProtectionDraft.keys.also{require(version==3)}
                else -> error("当前尚不支持这项管理操作")
            }
            require(values.keys == setOf("id", "action") + extra) { "参数与操作类型不匹配" }
            val id = value("id"); require(BinanceManageSnapshot.validId(id)) { "策略编号不可用" }
            val amount = if(action == "investment") BinanceInvestment.amount(value("amount")) else ""
            val range = if(action == "range") {
                val count = value("count"); require(Regex("[1-9][0-9]{0,4}").matches(count)) { "请输入有效格数" }
                BinanceRangeDraft(BinanceRange.price(value("lower")), BinanceRange.price(value("upper")),
                    count.toInt(), flag("close_positions"), value("amount"))
            } else null
            val protection=if(action=="protection")BinanceProtectionDraft.parse(BinanceProtectionDraft.keys.associateWith { if(it=="tpsl_cps")flag(it) else value(it) }) else null
            return BinanceManageCommandDraft(id,action,if(action in setOf("settings","close")) flag("cps") else false,amount,range,protection)
        }
    }
}

internal object BinanceManageCommandView {
    fun mode(cps: Boolean) = if(cps) "按市价平仓" else "保留仓位，由本人处理"
    fun action(value: String) = when(value) {
        "settings" -> "修改终止时仓位处理"; "close" -> "结束网格"
        "investment" -> "追加策略投入"; "range" -> "修改区间和格数"
        "protection" -> "修改止盈止损"; else -> "尚未选择"
    }
    fun detail(s: BinanceManageSnapshot,version:Int=2): Map<String,Any?> = linkedMapOf(
        "id" to s.id, "symbol" to s.symbol, "status" to s.status, "cps" to s.cps, "cos" to s.cos,
        "investment" to s.investment?.invested(), "range" to s.range?.let {
            mapOf("lower" to it.lower,"upper" to it.upper,"count" to it.count.toString(),
                "stop_lower" to it.preserved["stopLowerLimit"],"stop_upper" to it.preserved["stopUpperLimit"])
        }).apply {if(version==3)put("protection",s.protection?.publicDetail(s.investment))}
    fun summary(state: BinanceManageState) = buildString {
        val s = state.snapshot ?: return@buildString
        append("${s.symbol} · 策略 ${s.id}\n当前状态：${s.status}\n终止时：${mode(s.cps)}\n")
        append("取消合约委托：${if(s.cos) "是，请核对作用范围" else "否，需本人核对委托"}\n")
        s.investment?.let { append("参考累计投入：${it.invested()} USDT\n") }
        s.range?.let { append("当前区间：${it.lower} ～ ${it.upper} · ${it.count} 格\n原价格止盈止损：${it.preserved["stopLowerLimit"] ?: "未设置"} / ${it.preserved["stopUpperLimit"] ?: "未设置"}\n") }
        s.protection?.let {append("当前保护：${if(it.current().mode=="CLEAR")"未设置止盈止损" else it.current().description()}\n")}
        if(state.status != "prepared") return@buildString
        append("\n本次操作：${action(state.action)}\n")
        when(state.action) {
            "protection" -> state.protectionDraft?.let {
                append("${state.protectionInput?.description()}\n")
                if(state.protectionInput?.mode=="ROI")append("换算基数参考：${s.investment?.invested()} USDT\n最终金额：${it.description()}\n")
                append("触发：${if(it.stopType=="MARK_PRICE")"标记价格" else "最新合约价格"}；触发后${mode(it.closePositions)}\n")
                append("本次替换全部止盈止损；未填写项将清除。原网格区间、投入、追踪与普通终止设置保留。触发条件已满足时可能立即结束策略。")
            }
            "range" -> state.rangeDraft?.let {
                append("新区间：${it.lower} ～ ${it.upper} · ${it.count} 格\n现有仓位：${mode(it.closePositions)}\n追加：${it.investmentDelta} USDT\n")
                append("原止盈止损保持不变；将重建网格委托，最低追加金额由币安检查。")
            }
            "investment" -> append("追加：${state.investmentDelta} USDT\n追加后参考累计投入：${state.investmentPreview()} USDT\n追加投入不等于设置亏损上限。")
            "settings" -> append("终止时改为：${mode(state.cps)}。仅改设置，不立即结束；其他设置保持。")
            "close" -> append("按当前已确认设置结束：${mode(state.cps)}。不会先修改设置；受理或结束状态不证明仓位归零、委托全部撤销或资金结清。")
        }
    }
    fun digest(state: BinanceManageState) = BinanceHostState.digest(StrictJson.encode(linkedMapOf(
        "action" to state.action,"id" to state.id,"cps" to state.cps,"amount" to state.investmentDelta,
        "range" to state.rangeDraft?.payload(),"protection" to state.protectionDraft?.payload(),"summary" to summary(state))))
}
