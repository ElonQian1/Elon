package com.elon.app.grid.wallet

import android.content.Context
import android.os.Bundle
import android.os.SystemClock
import com.elon.app.grid.host.BinanceHostConsent
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.privateaccess.StrictJson
import java.security.SecureRandom

/** Uses the existing document and event channel; no polling or dependency on a grid row. */
internal class BinanceWalletRuntime(context:Context,private val host:BinanceHostRuntime) {
    val state=BinanceWalletState(SystemClock::elapsedRealtime,System::currentTimeMillis)
    private val consent=BinanceHostConsent(context,BinanceWalletState.PURPOSE)
    private var resumeGrant:String?=null
    private var waitingIdentity=false
    private var timeout:Runnable?=null
    var onChanged:(()->Unit)?=null
    private fun changed(){onChanged?.invoke();host.events.changed("read")}
    private fun permitted()=host.live()&&consent.permits(host.owner(),state.account,state.accountKind)
    fun clearSession() {
        timeout?.let(host.handler::removeCallbacks);timeout=null
        waitingIdentity=waitingIdentity || onChanged!=null || consent.recorded()
        state.clear();resumeGrant=null;changed()
    }
    fun disconnect() {consent.clear();waitingIdentity=false;clearSession();waitingIdentity=false;changed()}
    fun observeAccount(raw:String,kind:String) {
        if(!host.live())return
        val different=state.bind(raw,kind)
        if(different) {
            resumeGrant=null
            if(consent.recorded()&&!permitted())consent.clear()
            changed()
        }
        if(waitingIdentity&&state.identityFresh()){waitingIdentity=false;changed()}
    }
    fun identify(retry:Boolean=false) {
        if(!host.live()&&!host.begin())return
        if(state.identityFresh()||state.pending!=null)return
        if(state.status=="error"&&!retry)return
        waitingIdentity=true;documentReady()
    }
    fun documentReady() {
        if(!waitingIdentity || !host.live() || !host.adapterBound || state.pending!=null || state.identityFresh())return
        dispatch(newId(),"identify")
    }
    fun contextObserved(){if(waitingIdentity)documentReady()}
    fun approve():String {
        require(host.live()&&state.identityFresh()&&!state.identifying)
        consent.approve(host.owner()?:error("OWNER_MISSING"),state.account?:error("ACCOUNT_MISSING"),state.accountKind)
        return state.grant().also {resumeGrant=it}
    }
    fun resume():Bundle {
        fun status(value:String)=Bundle().apply {putString("status",value)}
        if(!consent.recorded())return status("consent_required")
        if(!host.live()&&!host.begin())return status("login_required")
        if(!state.identityFresh()||state.identifying) {
            identify();return status(if(state.status=="error")"read_unavailable" else "pending")
        }
        if(!permitted()){disconnect();return status("consent_required")}
        val grant=resumeGrant?.takeIf(state::authorized)?:state.grant().also {resumeGrant=it}
        return status("ready").apply {putString("grant",grant);putString("schema","yilong.binance_wallet_grant.v1")}
    }
    private fun authorize(grant:String) {
        require(permitted()&&state.authorized(grant));state.renew(grant);require(host.keepAlive())
    }
    fun refresh(grant:String,request:String):String {
        authorize(grant);require(BinanceWalletState.ID.matches(request))
        state.pending?.let {return it}
        dispatch(request,"read",grant);return request
    }
    fun read(grant:String):String {authorize(grant);return state.reply(grant)}
    private fun dispatch(request:String,kind:String,grant:String?=null) {
        val page=host.view?:return
        state.start(request,kind,grant)
        val doc=host.document.snapshot().documentToken
        val query=mapOf("kind" to kind,"request" to request)+(if(kind=="read")mapOf("account" to state.account)else emptyMap())
        val deadline=Runnable {if(host.document.accept(doc)!=null&&state.fail(request,"timeout"))changed()}
        timeout?.let(host.handler::removeCallbacks);timeout=deadline;host.handler.postDelayed(deadline,31_000)
        page.evaluateJavascript("window.__elonBinanceReadV1?.wallet(${StrictJson.encode(query)})") {result->
            if(host.document.accept(doc)!=null&&result!="true"&&state.fail(request,"context_unavailable"))changed()
        }
        changed()
    }
    fun observed(event:Map<String,Any?>) {
        if(!host.live() || host.document.accept(event["token"] as? String?:"")==null)return
        val request=event["request"] as? String?:return
        val accepted=runCatching {state.accept(event)}.getOrElse {state.fail(request,"response_invalid")}
        if(accepted){timeout?.let(host.handler::removeCallbacks);timeout=null;waitingIdentity=false;changed()}
    }
    companion object {
        fun newId()=ByteArray(32).also {SecureRandom().nextBytes(it)}.joinToString(""){"%02x".format(it)}
    }
}
