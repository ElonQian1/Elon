package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson

/** One on-demand page. An empty successful response, not a short page, proves the end. */
internal class OkxHistoryPage(val after: String, val rows: List<Map<String, Any?>>, val nextAfter: String?) {
    fun encode(account: OkxAccount, generation: Long, revision: Long, now: Long, bots: List<Map<String, Any?>>): String {
        val json = StrictJson.encode(mapOf("schema" to SCHEMA, "environment" to "live",
            "account" to account.reference, "account_kind" to account.kind, "generation" to generation,
            "revision" to revision, "observed_at_ms" to now, "fresh_until_ms" to now + 60_000,
            "after" to after, "next_after" to nextAfter, "complete" to (nextAfter == null), "bots" to bots))
        if (json.toByteArray(Charsets.UTF_8).size > 196_608) okxFail(OkxReadFailure.RESPONSE_LIMIT)
        return json
    }
    override fun toString() = "OkxHistoryPage(private)"
    companion object { const val SCHEMA = "yilong.okx_host_history.v1" }
}

internal class OkxHistoryReader(private val gateway: OkxReadGateway) {
    fun read(credentials: OkxCredentials, after: String): OkxHistoryPage {
        val request = OkxReadRequest.History.of(after)
        val rows = OkxReadProtocol.rows(gateway.get(credentials, request), 50)
        val seen = mutableSetOf<String>()
        var oldest: String? = null
        for (row in rows) {
            val id = row["algoId"] as? String ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
            if (!Regex("[1-9][0-9]{0,63}").matches(id) || !seen.add(id) || row["state"] != "stopped" ||
                (after.isNotEmpty() && !OkxReadProtocol.older(id, after))) okxFail(OkxReadFailure.INVALID_RESPONSE)
            if (oldest == null || OkxReadProtocol.older(id, oldest)) oldest = id
        }
        return OkxHistoryPage(after, rows, oldest)
    }
}
