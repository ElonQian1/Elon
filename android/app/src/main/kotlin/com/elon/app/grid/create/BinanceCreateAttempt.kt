package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson

/** Single-use attempt; the journal deliberately contains no credentials or financial parameters. */
internal class BinanceCreateAttempt(private val elapsed: () -> Long) {
    var status = "idle"; private set
    var account = ""; private set
    var document = ""; private set
    var clientId = ""; private set
    var strategyId = ""; private set
    var providerStatus = ""; private set
    var code = ""; private set
    var windowCount = 0; private set
    var draft: BinanceGridDraft? = null; private set
    private var expires = 0L
    val unresolved get() = status in setOf("submitting", "unknown", "accepted", "observed")
    fun prepare(account: String, document: String, draft: BinanceGridDraft, clientId: String, windowCount: Int) {
        require(!unresolved && Regex("[a-f0-9]{64}").matches(account) && Regex("doc_[a-z0-9_]{3,80}").matches(document))
        require(Regex("[A-Za-z0-9_-]{1,32}").matches(clientId) && windowCount in 1..10000)
        this.account = account; this.document = document; this.clientId = clientId; this.draft = draft
        this.windowCount = windowCount; expires = elapsed() + 60_000
        strategyId = ""; providerStatus = ""; code = ""; status = "prepared"
    }
    fun canSubmit(account: String?, document: String) = status == "prepared" && draft != null &&
        elapsed() < expires && account == this.account && document == this.document
    fun start(account: String?, document: String) {
        require(canSubmit(account, document)); status = "submitting"
    }
    fun invalidatePreparation() { if (status == "prepared") { status = "idle"; draft = null } }
    fun unknown() { if (status == "submitting") status = "unknown" }
    fun notSent(reason: String) { require(status in setOf("submitting", "unknown")); code = safeCode(reason); status = "not_sent" }
    fun rejected(reason: String) { require(status in setOf("submitting", "unknown")); code = safeCode(reason); status = "rejected" }
    fun accepted(id: String, rawStatus: String) {
        require(status in setOf("submitting", "unknown"))
        require(Regex("[0-9]{1,20}").matches(id)); require(Regex("[A-Z][A-Z0-9_]{0,63}").matches(rawStatus))
        strategyId = id; providerStatus = rawStatus; status = "accepted"
    }
    fun detail(id: String, rawStatus: String) {
        require(status in setOf("accepted", "observed") && id == strategyId)
        require(Regex("[A-Z][A-Z0-9_]{0,63}").matches(rawStatus)); providerStatus = rawStatus; status = "observed"
    }
    fun journal() = StrictJson.encode(mapOf("schema" to "yilong.binance_create_journal.v1", "status" to status,
        "account" to account, "client_id" to clientId, "strategy_id" to strategyId, "provider_status" to providerStatus))
    fun restore(raw: String) {
        val v = StrictJson.parse(raw, 2048)
        require(v.keys == setOf("schema", "status", "account", "client_id", "strategy_id", "provider_status"))
        require(v["schema"] == "yilong.binance_create_journal.v1")
        account = v["account"] as String; require(Regex("[a-f0-9]{64}").matches(account))
        clientId = v["client_id"] as String; require(Regex("[A-Za-z0-9_-]{1,32}").matches(clientId))
        strategyId = v["strategy_id"] as String; require(strategyId.isEmpty() || Regex("[0-9]{1,20}").matches(strategyId))
        providerStatus = v["provider_status"] as String; require(providerStatus.isEmpty() || Regex("[A-Z][A-Z0-9_]{0,63}").matches(providerStatus))
        require(v["status"] in setOf("submitting", "unknown", "accepted", "observed"))
        status = if (v["status"] in setOf("accepted", "observed") && strategyId.isNotEmpty()) "accepted" else "unknown"
        draft = null; document = ""; expires = 0
    }
    private fun safeCode(value: String): String { require(Regex("[A-Za-z0-9_-]{1,64}").matches(value)); return value }
}
