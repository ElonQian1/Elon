package com.elon.app.esk.platform.sellback

internal data class SellbackPolicy(
    val digest: String, val revision: String, val termsDigest: String, val terms: String,
    val minimum: Long, val maximum: Long, val maxOpen: Long, val maxReserved: Long, val recovery: String,
) { override fun toString() = "SellbackPolicy(redacted)" }

internal data class SellbackSummary(
    val digest: String, val total: Long, val reserved: Long, val available: Long,
    val openCount: Long, val count: Long, val enabled: Boolean, val reason: String, val policy: SellbackPolicy?,
) { override fun toString() = "SellbackSummary(redacted)" }

internal data class SellbackRecord(
    val id: String, val key: String, val amount: Long, val expectedDigest: String, val requestDigest: String,
    val policyRevision: String, val policyDigest: String, val termsDigest: String, val created: String,
    val canceled: String?, val cancelEvent: String?, val status: String,
) { override fun toString() = "SellbackRecord(redacted)" }

internal data class SellbackPage(
    val summary: SellbackSummary, val requests: List<SellbackRecord>, val start: Long,
    val end: Long, val nextCursor: String?,
) { override fun toString() = "SellbackPage(redacted)" }

internal data class SellbackResult(val summary: SellbackSummary, val request: SellbackRecord, val replayed: Boolean) {
    override fun toString() = "SellbackResult(redacted)"
}

internal fun sellbackAmount(units: Long): String {
    require(units >= 0)
    return "${units / 1_000_000}.${(units % 1_000_000).toString().padStart(6, '0')}"
}

internal fun sellbackInput(raw: String): Long? {
    if (!Regex("(?:0|[1-9][0-9]{0,12})(?:\\.[0-9]{1,6})?").matches(raw)) return null
    val parts = raw.split('.')
    val digits = parts[0] + parts.getOrElse(1) { "" }.padEnd(6, '0')
    return digits.toLongOrNull()?.takeIf { it > 0 }
}
