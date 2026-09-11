package com.elon.app.grid.create

import java.math.BigDecimal

/** Additional V2 projection only. V1 does not accept these fields. */
internal object BinanceReferenceEconomics {
    val keys=setOf("profit_min","profit_max","quantity","quantity_unit","quantity_status")
    fun validate(data:Map<String,Any?>,symbol:String) {
        require(keys.all {data[it] is String})
        val min=data["profit_min"] as String;val max=data["profit_max"] as String
        val qty=data["quantity"] as String;val status=data["quantity_status"] as String
        require(status in setOf("count_unavailable","margin_required","margin_below_minimum","ready"))
        require(data["quantity_unit"] in setOf(symbol.removeSuffix("USDT"),"USDT"))
        require((data["code"]=="")==(status!="count_unavailable"))
        if(status=="count_unavailable")require(min.isEmpty() && max.isEmpty()) else {
            require(decimal(min,true) && decimal(max,true) && BigDecimal(min)<=BigDecimal(max))
        }
        if(status=="ready")require(decimal(qty,false) && BigDecimal(qty)>BigDecimal.ZERO) else require(qty.isEmpty())
    }
    private fun decimal(value:String,signed:Boolean)=Regex((if(signed)"-?" else "")+"(0|[1-9][0-9]{0,39})(\\.[0-9]{1,40})?").matches(value)
}
