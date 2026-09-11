package com.elon.app.grid.manage

import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import java.math.RoundingMode

/** Positive loss magnitude, as used by Binance module 24012; ROI is never sent on the wire. */
internal data class BinanceProtectionDraft(val mode:String,val lower:String,val upper:String,val tp:String,val sl:String,
    val stopType:String,val closePositions:Boolean) {
    init {
        require(mode in setOf("PRICE","PNL","ROI","CLEAR")) {"请选择止盈止损方式。"}
        require(stopType in setOf("MARK_PRICE","CONTRACT_PRICE"))
        require(if(mode=="PRICE") tp.isEmpty() && sl.isEmpty() else lower.isEmpty() && upper.isEmpty()) {"不能同时设置价格与盈亏保护。"}
        require(if(mode=="CLEAR") listOf(lower,upper,tp,sl).all(String::isEmpty) else listOf(lower,upper,tp,sl).any(String::isNotEmpty)) {"请设置至少一项，或明确选择清除保护。"}
        listOf(lower,upper,tp,sl).filter(String::isNotEmpty).forEach {
            number(it,if(mode=="PNL")2 else 20)
        }
        require(lower.isEmpty() || upper.isEmpty() || BigDecimal(lower)<BigDecimal(upper)) {"保护下限必须小于上限。"}
    }
    fun normalize(investment:BinanceInvestmentSnapshot?):BinanceProtectionDraft {
        if(mode!="ROI") return copy(lower=canonical(lower),upper=canonical(upper),tp=canonical(tp),sl=canonical(sl))
        require(investment!=null) {"缺少已核验的投入，无法换算收益率。"}
        val leverage=BigDecimal(investment.initialLeverage)
        val numerator=BigDecimal(investment.initialValue).add(BigDecimal(investment.totalAdjustment).multiply(leverage))
        require(numerator.signum()>0) {"当前累计投入不为正，无法换算收益率。"}
        fun convert(value:String,rounding:RoundingMode):String {
            if(value.isEmpty())return ""
            val result=numerator.multiply(BigDecimal(value)).divide(leverage.multiply(BigDecimal("100")),2,rounding)
            require(result.signum()>0) {"收益率换算不足0.01 USDT，请调整设置。"}
            return number(result.stripTrailingZeros().toPlainString(),2)
        }
        return copy(mode="PNL",tp=convert(tp,RoundingMode.DOWN),sl=convert(sl,RoundingMode.UP))
    }
    fun payload():Map<String,Any> = linkedMapOf("mode" to mode,"lower" to lower,"upper" to upper,"tp" to tp,"sl" to sl,
        "stop_type" to stopType,"tpsl_cps" to closePositions)
    fun target()=BinanceHostState.digest(StrictJson.encode(payload().filterKeys{it!="mode"}))
    fun description()=when(mode) {
        "PRICE" -> "价格下限：${lower.ifEmpty{"未设置"}} / 上限：${upper.ifEmpty{"未设置"}} USDT"
        "CLEAR" -> "清除全部价格与盈亏止盈止损"
        else -> "止盈：${tp.ifEmpty{"未设置"}} / 止损：${sl.ifEmpty{"未设置"}} ${if(mode=="ROI")"%" else "USDT"}"
    }
    companion object {
        val keys=setOf("mode","lower","upper","tp","sl","stop_type","tpsl_cps")
        fun number(raw:String,scale:Int=20):String {
            require(Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,$scale})?").matches(raw) && BigDecimal(raw).signum()>0) {"请输入正数，金额最多2位小数；不支持指数格式。"}
            return BigDecimal(raw).stripTrailingZeros().toPlainString()
        }
        fun canonical(raw:String)=if(raw.isEmpty()) "" else number(raw)
        fun parse(raw:Any?):BinanceProtectionDraft {
            val v=raw as? Map<*,*> ?: error("保护参数不完整")
            require(v.keys==keys)
            return BinanceProtectionDraft(v["mode"] as String,v["lower"] as String,v["upper"] as String,v["tp"] as String,
                v["sl"] as String,v["stop_type"] as String,v["tpsl_cps"] as Boolean)
        }
    }
}
