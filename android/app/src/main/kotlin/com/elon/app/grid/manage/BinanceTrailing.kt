package com.elon.app.grid.manage

import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal

internal data class BinanceTrailingDraft(val upPrice:String,val downPrice:String) {
    init { for(v in listOf(upPrice,downPrice))require(v.isEmpty() || BinanceProtectionDraft.canonical(v)==v) }
    fun payload()=linkedMapOf("up_price" to upPrice,"down_price" to downPrice)
    fun target(before:BinanceTrailingSnapshot)=fingerprint(before.up,before.down,upPrice,downPrice)
    fun validate(before:BinanceTrailingSnapshot,rules:BinanceTrailingRules) {
        require(before.up || before.down)
        fun check(enabled:Boolean,value:String,bound:String,up:Boolean) {
            if(!enabled){require(value.isEmpty()){ "未开启的追踪方向不能填写停止价" };return}
            require(value.isNotEmpty()){ "请填写已开启方向的停止追踪价格" }
            val price=BigDecimal(value)
            require(price.remainder(BigDecimal(rules.tick)).signum()==0){"价格须符合 ${rules.tick} 的最小变动单位"}
            require(if(up)price>BigDecimal(bound) && price<=BigDecimal(rules.maximum)
                else price<BigDecimal(bound) && price>=BigDecimal(rules.minimum)){"停止追踪价格超出当前允许范围，请重新检查"}
        }
        check(before.up,upPrice,before.upper,true);check(before.down,downPrice,before.lower,false)
        require(target(before)!=before.target()){ "追踪停止价没有变化" }
    }
    companion object {
        fun parse(raw:Any?):BinanceTrailingDraft {
            val v=raw as? Map<*,*> ?: error("缺少追踪参数")
            require(v.keys==setOf("up_price","down_price") && v.values.all{it is String})
            fun price(key:String)=(v[key] as String).let{if(it.isEmpty())it else BinanceProtectionDraft.canonical(it)}
            return BinanceTrailingDraft(price("up_price"),price("down_price"))
        }
        fun fingerprint(up:Boolean,down:Boolean,upPrice:String,downPrice:String)=BinanceHostState.digest(StrictJson.encode(
            linkedMapOf("up" to up,"down" to down,"up_price" to upPrice,"down_price" to downPrice)))
    }
}

internal data class BinanceTrailingRules(val minimum:String,val maximum:String,val tick:String) {
    init {for(v in listOf(minimum,maximum,tick))require(BinanceProtectionDraft.canonical(v)==v);require(BigDecimal(minimum)<BigDecimal(maximum))}
    fun payload()=linkedMapOf("minimum" to minimum,"maximum" to maximum,"tick" to tick)
    companion object {
        fun parse(raw:Any?):BinanceTrailingRules {
            val v=raw as? Map<*,*> ?: error("追踪范围不可用")
            require(v.keys==setOf("minimum","maximum","tick"))
            return BinanceTrailingRules(v["minimum"] as String,v["maximum"] as String,v["tick"] as String)
        }
    }
}

internal data class BinanceTrailingSnapshot(val fields:Map<String,Any?>) {
    val up get()=fields["up"] as Boolean
    val down get()=fields["down"] as Boolean
    val upPrice get()=fields["up_price"] as String
    val downPrice get()=fields["down_price"] as String
    val lower get()=fields["lower"] as String
    val upper get()=fields["upper"] as String
    fun current()=BinanceTrailingDraft(upPrice,downPrice)
    fun target()=BinanceTrailingDraft.fingerprint(up,down,upPrice,downPrice)
    fun publicDetail(rules:BinanceTrailingRules)=fields.filterKeys{it !in setOf("count","type")}+rules.payload()
    fun baseline(s:BinanceManageSnapshot):Map<String,Any?> {
        require(s.trailing==this && s.range==null && s.protection!=null)
        return linkedMapOf("strategy_id" to s.id,"symbol" to s.symbol,"provider_status" to s.status,"cps" to s.cps,"cos" to s.cos,
            "sharing" to s.sharing,"trailingStopLowerLimit" to s.trailingLower,"trailingStopUpperLimit" to s.trailingUpper,
            "investment" to s.protection.baseline(s.investment)["investment"],"range" to null,"protection" to s.protection.fields,"trailing" to fields)
    }
    companion object {
        private val keys=setOf("up","down","up_price","down_price","lower","upper","count","type")
        fun parse(raw:Any?):BinanceTrailingSnapshot {
            val v=raw as? Map<*,*> ?: error("追踪设置不可用")
            require(v.keys==keys && v["up"] is Boolean && v["down"] is Boolean)
            val count=(v["count"] as? StrictJson.Number)?.text?.toIntOrNull() ?: error("追踪格数不可用")
            require(count in 2..10000 && v["type"] in setOf("ARITH","GEO"))
            val result=BinanceTrailingSnapshot(keys.associateWith{if(it=="count")count else v[it]})
            require(result.up || result.down)
            for(p in listOf(result.lower,result.upper))require(BinanceProtectionDraft.canonical(p)==p)
            require(BigDecimal(result.lower)<BigDecimal(result.upper))
            result.current();require(result.up || result.upPrice.isEmpty());require(result.down || result.downPrice.isEmpty())
            return result
        }
    }
}
