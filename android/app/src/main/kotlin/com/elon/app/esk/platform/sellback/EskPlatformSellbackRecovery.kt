package com.elon.app.esk.platform.sellback

import com.elon.app.esk.platform.EskPlatformSession

/** Warning identity only, including legacy sessions. Never grants permission to replay a write. */
internal class SellbackReviewIdentity(session: EskPlatformSession) {
    private val user = session.userId
    private val revision = session.revision
    private val expiry = session.expiresAtMillis
    fun belongsTo(session: EskPlatformSession) = user == session.userId && revision == session.revision && expiry == session.expiresAtMillis
    override fun toString() = "SellbackReviewIdentity(redacted)"
}

/** Process-memory read-only lookup hint. No token, amount, payload, or prior consent is retained. */
internal object EskPlatformSellbackRecovery {
    internal class Hint(val key: String, val cancel: Boolean, private val user: String,
        private val revision: String, private val expiry: Long) {
        fun belongsTo(session: EskPlatformSession) = user == session.userId && revision == session.revision && expiry == session.expiresAtMillis
        fun resolvedBy(record: SellbackRecord) = record.key == key && (!cancel || record.status == "canceled")
        override fun toString() = "SellbackRecoveryHint(redacted)"
    }
    private var hint: Hint? = null
    @Synchronized fun remember(action: SellbackAction, session: EskPlatformSession) {
        // A legacy null revision cannot identify a replaced token without retaining a credential.
        hint = session.revision?.let {
            Hint(requireNotNull(action.key), !action.isSubmit, session.userId, it, session.expiresAtMillis)
        }
    }
    @Synchronized fun current(session: EskPlatformSession): Hint? {
        if (hint?.belongsTo(session) == false) hint = null
        return hint
    }
    @Synchronized fun resolve(session: EskPlatformSession, records: List<SellbackRecord>): Boolean {
        val expected = current(session) ?: return false
        if (records.none(expected::resolvedBy)) return false
        hint = null
        return true
    }
    @Synchronized fun clear() { hint = null }
}
