package com.elon.app.esk.platform.sellback

import com.elon.app.esk.platform.EskPlatformSession
import com.google.gson.Gson

/** Immutable bytes are reused after an unknown result; never synthesize a replacement key. */
internal class SellbackAction private constructor(
    val requestId: String?, val key: String?, val amount: Long, val terms: String,
    val body: String, private val expected: Map<String, String>,
) {
    val isSubmit: Boolean get() = requestId == null
    fun matches(record: SellbackRecord): Boolean = if (isSubmit) {
        record.key == key && record.amount == amount && record.expectedDigest == expected["expected_snapshot_digest"] &&
            record.policyDigest == expected["policy_digest"] && record.termsDigest == expected["terms_digest"]
    } else record.id == requestId && record.amount == amount && record.status == "canceled"
    override fun toString() = "SellbackAction(redacted)"

    companion object {
        fun submit(summary: SellbackSummary, amount: Long, key: String): SellbackAction {
            val policy = requireNotNull(summary.policy)
            require(summary.enabled && amount in policy.minimum..policy.maximum && amount <= summary.available)
            require(summary.openCount < policy.maxOpen && summary.reserved <= policy.maxReserved && amount <= policy.maxReserved - summary.reserved)
            require(key.length in 1..96 && Regex("[A-Za-z0-9._:-]+").matches(key))
            val fields = sortedMapOf("schema" to "yilong.esk.platform_sellback_submit.v1", "idempotency_key" to key,
                "amount_base_units" to amount.toString(), "expected_snapshot_digest" to summary.digest,
                "policy_digest" to policy.digest, "terms_digest" to policy.termsDigest,
                "confirmation" to "SUBMIT PLATFORM ESK SELLBACK REQUEST")
            return SellbackAction(null, key, amount, policy.terms, Gson().toJson(fields), fields.toMap())
        }
        fun cancel(record: SellbackRecord): SellbackAction {
            require(record.status == "submitted" && EskPlatformSellbackParser.validId(record.id))
            val fields = sortedMapOf("schema" to "yilong.esk.platform_sellback_cancel.v1",
                "confirmation" to "CANCEL PLATFORM ESK SELLBACK REQUEST")
            return SellbackAction(record.id, record.key, record.amount, "取消仅解除这笔申请占用，不成交、不返币、不付款。",
                Gson().toJson(fields), fields.toMap())
        }
    }
}

/** Confirmation authority is foreground-only. A transport cancellation is never a business rollback. */
internal class EskPlatformSellbackState {
    internal class Draft(val action: SellbackAction, internal val session: EskPlatformSession, internal val at: Long) {
        override fun toString() = "SellbackDraft(redacted)"
    }
    internal class Ticket(internal val draft: Draft) { override fun toString() = "SellbackTicket(redacted)" }
    private var draft: Draft? = null
    private var active: Ticket? = null
    private var uncertain: Draft? = null

    @Synchronized fun prepare(action: SellbackAction, session: EskPlatformSession, now: Long,
        epoch: Long, foreground: Boolean): Draft? {
        if (active != null || uncertain != null || !foreground || now < 0 || !session.validAt(epoch)) return null
        return Draft(action, session, now).also { draft = it }
    }
    @Synchronized fun confirm(candidate: Draft, current: EskPlatformSession?, now: Long,
        epoch: Long, foreground: Boolean): Ticket? {
        if (draft !== candidate) return null
        draft = null
        if (!foreground || !candidate.session.sameAs(current) || current?.validAt(epoch) != true ||
            now < candidate.at || now - candidate.at >= 60_000L) return null
        return Ticket(candidate).also { active = it }
    }
    @Synchronized fun dismiss(candidate: Draft) { if (draft === candidate) draft = null }
    @Synchronized fun unknown(ticket: Ticket): Boolean {
        if (active !== ticket) return false
        active = null
        uncertain = ticket.draft
        return true
    }
    @Synchronized fun complete(ticket: Ticket): Boolean {
        if (active !== ticket) return false
        active = null; uncertain = null; draft = null
        return true
    }
    @Synchronized fun retry(current: EskPlatformSession?, now: Long, epoch: Long, foreground: Boolean): Draft? {
        val original = uncertain ?: return null
        if (active != null || !foreground || now < 0 || !original.session.sameAs(current) || current?.validAt(epoch) != true) return null
        return Draft(original.action, current, now).also { draft = it }
    }
    @Synchronized fun unresolved(): Boolean = uncertain != null || active != null
    @Synchronized fun resolve(records: List<SellbackRecord>): Boolean {
        val original = uncertain ?: return false
        if (records.none(original.action::matches)) return false
        uncertain = null; draft = null
        return true
    }
    @Synchronized fun clear(): Boolean {
        val unknown = active != null || uncertain != null
        draft = null; active = null; uncertain = null
        return unknown
    }
}
