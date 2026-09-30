package com.elon.app.esk.compute

import java.math.BigInteger

internal data class CenterRow(val title: String, val amount: String, val details: List<Pair<String, String>>)
internal data class CenterQuote(val source: String, val observedAt: Long, val validUntil: Long, val usdt: String?, val cny: String?)
internal data class EskComputeSnapshot(
    val observedAt: Long, val freshUntil: Long, val total: String, val reserved: String,
    val remaining: String, val entryCount: String, val quote: CenterQuote?, val balance: String?,
    val monthCost: String, val purchases: List<CenterRow>, val usage: List<CenterRow>,
    val bills: List<CenterRow>, val holds: List<CenterRow>, val page: Int,
    val billsHasMore: Boolean, val purchasesHaveMore: Boolean, val holdsHaveMore: Boolean,
) { override fun toString() = "EskComputeSnapshot(redacted)" }

internal fun natural(value: String): BigInteger {
    require(Regex("^(0|[1-9][0-9]*)$").matches(value) && value.length <= 19)
    return value.toBigInteger().also { require(it <= BigInteger.valueOf(Long.MAX_VALUE)) }
}
internal fun amount(value: String, decimals: Int = 6): String {
    val signed = value.startsWith('-')
    val absolute = natural(if (signed) value.substring(1) else value)
    require(!signed || absolute.signum() > 0)
    val parts = absolute.divideAndRemainder(BigInteger.TEN.pow(decimals))
    return (if (signed) "-" else "") + parts[0].toString() + "." + parts[1].toString().padStart(decimals, '0')
}
