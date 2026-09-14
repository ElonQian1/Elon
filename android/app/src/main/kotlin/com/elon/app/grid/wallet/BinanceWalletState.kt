package com.elon.app.grid.wallet

import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import java.security.SecureRandom

/** Main-looper state; wallet authority is separate from every grid grant. */
internal class BinanceWalletState(private val elapsed:()->Long,private val epoch:()->Long) {
    var account:String?=null;private set
    var accountKind="unknown";private set
    var status="idle";private set
    var error:String?=null;private set
    var pending:String?=null;private set
    private var pendingKind=""
    private var pendingAccount:String?=null
    private var pendingGrant:String?=null
    private var identityAt=Long.MIN_VALUE
    private var observedAt=0L
    private var observedElapsed=0L
    private var generation=0L
    private var lastRequest=""
    private var wallets:List<Map<String,Any?>> = emptyList()
    private val grants=mutableMapOf<String,Long>()
    val walletCount get()=wallets.size
    val identifying get()=pending!=null&&pendingKind=="identify"
    fun identityRemainingMs():Long {
        if(account==null)return 0
        val age=elapsed()-identityAt
        return if(age in 0 until 300_000)300_000-age else 0
    }
    fun identityFresh()=identityRemainingMs()>0
    fun bind(raw:String,kind:String):Boolean {
        require(Regex("[0-9]{1,20}").matches(raw) && kind in setOf("primary","sub","unknown"))
        val next=BinanceHostState.digest(raw);val changed=account!=next || accountKind!=kind
        if(changed) {
            val identifyingRequest=pending.takeIf {pendingKind=="identify"}
            clear()
            if(identifyingRequest!=null){pending=identifyingRequest;pendingKind="identify";status="pending"}
        }
        account=next;accountKind=kind;identityAt=elapsed()
        return changed
    }
    fun clear() {
        account=null;accountKind="unknown";identityAt=Long.MIN_VALUE;grants.clear();wallets=emptyList()
        pending=null;pendingKind="";pendingAccount=null;pendingGrant=null
        status="idle";error=null;observedAt=0;observedElapsed=0;lastRequest="";generation++
    }
    fun grant():String {
        require(identityFresh());grants.entries.removeAll {it.value<=elapsed()};require(grants.size<8)
        val token=ByteArray(32).also {SecureRandom().nextBytes(it)}.joinToString(""){"%02x".format(it)}
        grants[token]=Math.addExact(elapsed(),900_000);return token
    }
    fun remaining(token:String)=((grants[token]?:0)-elapsed()).coerceAtLeast(0)
    fun authorized(token:String)=ID.matches(token)&&account!=null&&remaining(token)>0
    fun renew(token:String){require(authorized(token));grants[token]=Math.addExact(elapsed(),900_000)}
    fun start(request:String,kind:String,grant:String?=null) {
        require(ID.matches(request) && kind in setOf("identify","read") && pending==null)
        if(kind=="read")require(grant!=null&&authorized(grant))
        pending=request;pendingKind=kind;pendingAccount=account;pendingGrant=grant;status="pending";error=null
    }
    fun fail(request:String,code:String):Boolean {
        if(pending!=request)return false
        if(code=="account_changed" || code=="identity_unverified")clear()
        pending=null;pendingGrant=null;status="error";error=code.takeIf {it in ERRORS}?:"read_failed";wallets=emptyList();generation++
        return true
    }
    fun accept(event:Map<String,Any?>):Boolean {
        if(event["request"]!=pending || pending==null)return false
        require(event["schema"]==OBSERVATION)
        val request=pending!!
        if(event["kind"]=="error")return fail(request,event["error"] as? String?:"read_failed")
        val raw=event["account"] as? String?:error("ACCOUNT_MISSING")
        val kind=event["account_kind"] as? String?:error("ACCOUNT_KIND_MISSING")
        if(pendingKind=="identify") {
            require(event["kind"]=="identity");bind(raw,kind);status="identity_ready"
        } else {
            require(event["kind"]=="wallet" && event["quote_asset"]=="USDT")
            require(BinanceHostState.digest(raw)==pendingAccount && kind==accountKind && pendingGrant?.let(::authorized)==true)
            val rows=event["wallets"] as? List<*>?:error("WALLETS_MISSING")
            require(rows.size<=64)
            val decoded=rows.map {value->
                val row=value as? Map<*,*>?:error("WALLET_INVALID")
                require(row.keys==setOf("type","active","balance"))
                val type=row["type"] as? String?:error("TYPE_MISSING")
                require(Regex("[A-Z][A-Z0-9_]{0,47}").matches(type) && row["active"] is Boolean)
                val amount=row["balance"]
                require(amount==null || amount is String&&DECIMAL.matches(amount))
                mapOf("type" to type,"active" to row["active"],"balance" to amount)
            }
            require(decoded.map {it["type"]}.toSet().size==decoded.size)
            wallets=decoded;observedAt=epoch();observedElapsed=elapsed();identityAt=elapsed();status="fresh";lastRequest=request
        }
        pending=null;pendingGrant=null;error=null;generation++;return true
    }
    fun reply(token:String):String {
        require(authorized(token))
        val effective=if(status=="fresh"&&elapsed()-observedElapsed !in 0 until 300_000)"stale" else status
        return StrictJson.encode(mapOf("schema" to SCHEMA,"source" to "android_webview","account" to account,
            "account_kind" to accountKind,"quote_asset" to "USDT","generation" to generation,
            "request_id" to (pending?:lastRequest),"observed_at_ms" to observedAt,"remaining_ms" to remaining(token),
            "status" to effective,"error" to error,"coverage" to "observed_wallet_response_only","wallets" to wallets))
    }
    companion object {
        const val PURPOSE="wallet_summary_read"
        const val SCHEMA="yilong.binance_wallet_read.v1"
        const val OBSERVATION="yilong.binance_wallet_observation.v1"
        val ID=Regex("[a-f0-9]{64}")
        private val DECIMAL=Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?")
        private val ERRORS=setOf("account_changed","identity_unverified","rate_limited","http_failed","business_failed","response_invalid","response_too_large","read_failed","timeout","context_unavailable")
    }
}
