package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson

/** Currency-level USDT values, never account-level USD valuations or a derived trading limit. */
internal object OkxBalance {
    const val SCHEMA = "yilong.okx_account_balance.v1"
    private val decimal = Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?")
    fun decode(raw: String): Map<String, String?> {
        val account = OkxReadProtocol.rows(raw, 1).singleOrNull() ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
        val details = account["details"] as? List<*> ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
        require(details.size <= 1000)
        val rows = details.map { it as? Map<*, *> ?: error("INVALID_BALANCE") }.filter { it["ccy"] == "USDT" }
        require(rows.size <= 1)
        fun amount(key: String): String? {
            val value = rows.singleOrNull()?.get(key) ?: return null
            require(value is String)
            if (value.isEmpty()) return null
            require(decimal.matches(value)); return value
        }
        return mapOf("available" to amount("availBal"), "equity" to amount("eq"), "frozen" to amount("frozenBal"))
    }
    fun encode(account: OkxAccount, generation: Long, now: Long, amounts: Map<String, String?>): String =
        StrictJson.encode(mapOf("schema" to SCHEMA, "account" to account.reference, "account_kind" to account.kind,
            "generation" to generation, "observed_at_ms" to now, "quote_asset" to "USDT") + amounts)
}
