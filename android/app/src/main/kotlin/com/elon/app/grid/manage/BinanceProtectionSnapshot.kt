package com.elon.app.grid.manage

/** Complete editable baseline, separate from ordinary end-of-grid cps and range editing. */
internal data class BinanceProtectionSnapshot(val fields:Map<String,Any?>) {
    val tp get()=fields["tp"] as String
    val sl get()=fields["sl"] as String
    fun current()=BinanceProtectionDraft(if(tp.isNotEmpty() || sl.isNotEmpty())"PNL" else
        if(fields["lower"]!="" || fields["upper"]!="")"PRICE" else "CLEAR",
        fields["lower"] as String,fields["upper"] as String,tp,sl,fields["stop_type"] as String,fields["tpsl_cps"] as Boolean)
    fun target()=current().normalize(null).target()
    fun baseline(investment:BinanceInvestmentSnapshot?)=linkedMapOf("protection" to fields,"investment" to investment?.let {
        linkedMapOf("initial_value" to it.initialValue,"initial_leverage" to it.initialLeverage,"total_adjustment" to it.totalAdjustment)
    })
    fun publicDetail(investment:BinanceInvestmentSnapshot?)=fields.filterKeys{it in publicKeys} + mapOf("roi_base" to investment?.invested())
    companion object {
        private val publicKeys=setOf("direction","stop_type","lower","upper","tp","sl","tpsl_cps","trailing_up","trailing_down")
        private val keys=publicKeys+setOf("trigger_type","trigger_price","auto_init","trailing_up_price","trailing_down_price")
        fun parse(raw:Any?):BinanceProtectionSnapshot {
            val v=raw as? Map<*,*> ?: error("保护详情不可用")
            require(v.keys==keys && v["direction"] in setOf("LONG","SHORT","NEUTRAL"))
            for(k in listOf("stop_type","trigger_type"))require(v[k] in setOf("MARK_PRICE","CONTRACT_PRICE"))
            for(k in listOf("lower","upper","tp","sl","trigger_price","trailing_up_price","trailing_down_price")) {
                val value=v[k] as String;require(value.isEmpty() || BinanceProtectionDraft.canonical(value)==value)
            }
            for(k in listOf("tpsl_cps","trailing_up","trailing_down"))require(v[k] is Boolean)
            require(v["auto_init"]==null || v["auto_init"] is Boolean)
            val result=BinanceProtectionSnapshot(keys.associateWith{v[it]})
            result.current();return result
        }
    }
}
