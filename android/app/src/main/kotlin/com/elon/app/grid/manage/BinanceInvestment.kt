package com.elon.app.grid.manage

import java.math.BigDecimal
import java.math.RoundingMode
import java.security.MessageDigest
import com.elon.app.privateaccess.StrictJson

/** Explicit additional strategy margin, distinct from position margin or leveraged notional. */
internal object BinanceInvestment {
    fun amount(raw:String):String {
        require(Regex("(0|[1-9][0-9]{0,3})(\\.[0-9]{1,8})?").matches(raw)) {"请输入最多8位小数的追加金额。"}
        val value=BigDecimal(raw)
        require(value.signum()>0 && value<=BigDecimal("2000")) {"本次自有资金验证的追加金额须大于0且不超过2,000 USDT。"}
        return value.stripTrailingZeros().toPlainString()
    }
    fun decimal(raw:Any?):String {
        val value=raw as? String ?: error("缺少精确金额")
        require(Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(value))
        return BigDecimal(value).stripTrailingZeros().toPlainString()
    }
    fun fingerprint(raw:String)=MessageDigest.getInstance("SHA-256").digest(decimal(raw).toByteArray(Charsets.UTF_8))
        .joinToString(""){"%02x".format(it)}
}

internal data class BinanceInvestmentSnapshot(val initialValue:String,val initialLeverage:Int,val totalAdjustment:String) {
    init {
        require(BigDecimal(BinanceInvestment.decimal(initialValue)).signum()>0)
        require(initialLeverage in 1..125)
        BinanceInvestment.decimal(totalAdjustment)
    }
    fun invested(delta:String="0"):String = BigDecimal(initialValue).divide(BigDecimal(initialLeverage),20,RoundingMode.DOWN)
        .add(BigDecimal(totalAdjustment)).add(BigDecimal(delta)).stripTrailingZeros().toPlainString()
    fun target(delta:String)=BinanceInvestment.fingerprint(BigDecimal(totalAdjustment).add(BigDecimal(delta)).toPlainString())
    companion object {
        fun parse(raw:Any?):BinanceInvestmentSnapshot {
            val v=raw as? Map<*,*> ?: error("投入详情不可用")
            require(v.keys==setOf("initial_value","initial_leverage","total_adjustment"))
            val leverage=(v["initial_leverage"] as? StrictJson.Number)?.text?.takeIf{Regex("[1-9][0-9]{0,2}").matches(it)}?.toIntOrNull() ?: error("杠杆格式不完整")
            return BinanceInvestmentSnapshot(BinanceInvestment.decimal(v["initial_value"]),leverage,BinanceInvestment.decimal(v["total_adjustment"]))
        }
    }
}
