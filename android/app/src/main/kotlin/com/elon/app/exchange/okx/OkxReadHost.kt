package com.elon.app.exchange.okx

import android.content.Context
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import java.security.SecureRandom
import java.util.concurrent.locks.ReentrantLock

/** One process host, owner-bound consent and epoch-bound results; HTTP never holds the state lock. */
internal class OkxReadHost(private val vault: OkxAccessVault, private val capture: () -> EskPlatformSession?,
    private val gateway: OkxReadGateway, private val elapsed: () -> Long = android.os.SystemClock::elapsedRealtime) {
    private val lock = Any()
    private val network = ReentrantLock()
    private var active: Access? = null
    private var generation = 0L
    private var revision = 0L
    private class Access(val token: String, val owner: EskPlatformSession, val saved: OkxSavedAccess, val generation: Long) {
        override fun toString() = "OkxAccess(private)"
    }
    class Verified internal constructor(internal val owner: EskPlatformSession, internal val saved: OkxSavedAccess) {
        val kind get() = saved.account.kind
        override fun toString() = "OkxVerified(private)"
    }
    private fun invalidate() { active = null; generation++ }
    private fun owner() = capture() ?: okxFail(OkxReadFailure.HOST_SESSION_REQUIRED)
    private fun valid(value: Access) {
        if (active !== value || !value.owner.sameAs(capture())) okxFail(OkxReadFailure.CONNECTION_CHANGED)
    }
    private fun token() = ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
    private fun issue(owner: EskPlatformSession, saved: OkxSavedAccess): String {
        invalidate(); active = Access(token(), owner, saved, generation)
        return active!!.token
    }
    fun verify(credentials: OkxCredentials): Verified = exclusive {
        val captured = owner()
        val account = OkxReadProtocol.account(gateway.get(credentials, OkxReadRequest.Account))
        if (!captured.sameAs(capture())) okxFail(OkxReadFailure.CONNECTION_CHANGED)
        Verified(captured, OkxSavedAccess(credentials, account))
    }
    fun approve(verified: Verified): String = synchronized(lock) {
        if (!verified.owner.sameAs(capture())) okxFail(OkxReadFailure.CONNECTION_CHANGED)
        vault.save(verified.owner.userId, verified.saved)
        issue(verified.owner, verified.saved)
    }
    fun resume(): String = synchronized(lock) {
        val current = owner()
        active?.takeIf { it.owner.sameAs(current) }?.let { return it.token }
        invalidate()
        val saved = vault.read(current.userId) ?: okxFail(OkxReadFailure.AUTHORIZATION_REQUIRED)
        issue(current, saved)
    }
    fun revoke(grant: String) = synchronized(lock) {
        val value = access(grant)
        invalidate(); vault.revoke(value.owner.userId)
    }
    private fun access(grant: String): Access {
        val value = active ?: okxFail(OkxReadFailure.AUTHORIZATION_REQUIRED)
        valid(value)
        if (value.token != grant) okxFail(OkxReadFailure.AUTHORIZATION_REQUIRED)
        return value
    }
    fun read(grant: String, id: String?): String = readBound(grant, id, null)
    fun history(grant: String, after: String): String = readBound(grant, null, after)
    fun records(grant:String,kind:String,id:String,symbol:String,after:String)=readBound(grant,null,null,OkxRecordQuery(kind,id,symbol,after))
    private fun readBound(grant: String, id: String?, historyAfter: String?, recordQuery:OkxRecordQuery?=null): String = exclusive {
        val captured = synchronized(lock) { access(grant) }
        val credentials = captured.saved.credentials
        fun verifyAccount() {
            val account = OkxReadProtocol.account(gateway.get(credentials, OkxReadRequest.Account))
            if (account.reference != captured.saved.account.reference || account.kind != captured.saved.account.kind) {
                synchronized(lock) { if (active === captured) invalidate() }
                okxFail(OkxReadFailure.ACCOUNT_CHANGED)
            }
            synchronized(lock) { valid(captured) }
        }
        verifyAccount()
        val records=recordQuery?.let{OkxRecordsReader(gateway).read(credentials,it,System.currentTimeMillis())}
        val history = historyAfter?.let { OkxHistoryReader(gateway).read(credentials, it) }
        val rows = if(records!=null)emptyList() else history?.rows ?: if (id == null) OkxPendingReader(gateway, elapsed).read(credentials) else {
            val request = OkxReadRequest.Detail.of(id)
            OkxReadProtocol.rows(gateway.get(credentials, request), 1).also {
                if (it.size != 1 || it.single()["algoId"] != id) okxFail(OkxReadFailure.INVALID_RESPONSE)
            }
        }
        verifyAccount()
        val now = System.currentTimeMillis()
        val bots = rows.map { OkxReadProjection.bot(it, captured.generation, now, id != null) }
        synchronized(lock) {
            valid(captured)
            val nextRevision = ++revision
            records?.encode(captured.saved.account,captured.generation,nextRevision,now)
                ?: history?.encode(captured.saved.account, captured.generation, nextRevision, now, bots)
                ?: OkxReadProjection.encode(captured.saved.account, captured.generation, nextRevision, now, bots, id)
        }
    }
    private fun <T> exclusive(action: () -> T): T {
        if (!network.tryLock()) okxFail(OkxReadFailure.BUSY)
        try { return action() } finally { network.unlock() }
    }
    companion object {
        @Volatile private var instance: OkxReadHost? = null
        fun get(context: Context): OkxReadHost = instance ?: synchronized(this) {
            instance ?: run {
                val reference = java.util.concurrent.atomic.AtomicReference<OkxReadHost>()
                val sessions = EskPlatformSessionStore(context.applicationContext) { reference.get()?.let { host -> synchronized(host.lock) { host.invalidate() } } }
                OkxReadHost(OkxCredentialVault(context.applicationContext), { sessions.capture() }, OkxReadTransport()).also {
                    reference.set(it); instance = it
                }
            }
        }
    }
}
