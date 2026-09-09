package com.elon.app.grid.manage

import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import java.security.MessageDigest

internal object BinanceRange {
    fun price(raw:Any?):String {
        val text=raw as? String ?: error("缺少精确价格")
        require(Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(text)){"价格须为非负十进制数。"}
        return BigDecimal(text).stripTrailingZeros().toPlainString()
    }
    fun count(raw:Any?):Int {
        val value=(raw as? StrictJson.Number)?.text ?: error("格数格式不完整")
        require(Regex("[1-9][0-9]{0,4}").matches(value))
        return value.toInt().also{require(it in 2..10000){"格数须在2到10,000之间。"}}
    }
    fun target(lower:String,upper:String,count:Int)=MessageDigest.getInstance("SHA-256")
        .digest("${price(lower)}|${price(upper)}|$count".toByteArray(Charsets.UTF_8)).joinToString(""){"%02x".format(it)}
}

internal data class BinanceRangeDraft(val lower:String,val upper:String,val count:Int,val closePositions:Boolean,val investmentDelta:String) {
    init {
        require(BinanceRange.price(lower)==lower && BinanceRange.price(upper)==upper)
        require(BigDecimal(lower).signum()>0 && BigDecimal(upper)>BigDecimal(lower)){"上限必须大于下限，且下限大于0。"}
        require(count in 2..10000){"格数须在2到10,000之间。"}
        require(Regex("(0|[1-9][0-9]{0,3})(\\.[0-9]{1,8})?").matches(investmentDelta) && BigDecimal(investmentDelta)<=BigDecimal("2000")){
            "请明确填写0到2,000 USDT的追加金额，最多8位小数。"}
    }
    fun payload():Map<String,Any> = linkedMapOf("lower" to lower,"upper" to upper,"count" to count,
        "close_positions" to closePositions,"investment_delta" to investmentDelta)
    fun target()=BinanceRange.target(lower,upper,count)
    companion object {
        fun parse(raw:Any?):BinanceRangeDraft {
            val v=raw as? Map<*,*> ?: error("缺少区间参数")
            require(v.keys==setOf("lower","upper","count","close_positions","investment_delta"))
            return BinanceRangeDraft(BinanceRange.price(v["lower"]),BinanceRange.price(v["upper"]),BinanceRange.count(v["count"]),
                v["close_positions"] as Boolean,v["investment_delta"] as String)
        }
    }
}

internal data class BinanceRangeSnapshot(val lower:String,val upper:String,val count:Int,val gridType:String,val direction:String,
    val preserved:Map<String,Any?>) {
    fun target()=BinanceRange.target(lower,upper,count)
    companion object {
        fun parse(raw:Any?):BinanceRangeSnapshot {
            val v=raw as? Map<*,*> ?: error("区间详情不可用")
            require(v.keys==setOf("lower","upper","count","grid_type","direction","preserved"))
            val lower=BinanceRange.price(v["lower"]);val upper=BinanceRange.price(v["upper"])
            require(BigDecimal(lower).signum()>0 && BigDecimal(upper)>BigDecimal(lower))
            val gridType=v["grid_type"] as String;require(gridType in setOf("ARITH","GEO"))
            val direction=v["direction"] as String;require(direction in setOf("LONG","SHORT","NEUTRAL"))
            val source=v["preserved"] as? Map<*,*> ?: error("原止盈止损设置缺失")
            require(source["tpslCps"] is Boolean && source.keys.all{it in setOf("tpslCps","stopUpperLimit","stopLowerLimit","trailingUpLimitPrice","trailingDownLimitPrice")})
            val preserved=source.entries.associate{(key,value)->key as String to if(key=="tpslCps" || value==null)value else BinanceRange.price(value)}
            return BinanceRangeSnapshot(lower,upper,BinanceRange.count(v["count"]),gridType,direction,preserved)
        }
    }
}
