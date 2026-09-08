package com.elon.app.grid.manage

import android.os.SystemClock
import com.elon.app.grid.create.BinanceCreateSlot
import com.elon.app.grid.host.BinanceHostRuntime

/** Authenticated MCP reads outside a management page. No journal writes or trade preparation. */
internal object BinanceHostReadDebug {
    private var selectedId:String?=null
    private var selectedAccount:String?=null
    private var session:BinanceManageSession?=null
    private var requestCount=0L
    private var hook:((String)->Unit)?=null
    fun active(host:BinanceHostRuntime)=hook!=null && host.onCreateObservation===hook
    private fun current(host:BinanceHostRuntime)=host.live() && host.state.fresh()
    private fun selection(host:BinanceHostRuntime):String? {
        if(!current(host) || host.state.account!=selectedAccount || selectedId?.let(host.state::contains)!=true)return null
        return selectedId
    }
    private fun occupied(host:BinanceHostRuntime)=host.onCreateObservation!=null && host.onCreateObservation!==hook
    private fun facts(host:BinanceHostRuntime):Map<String,Any?> {
        val fresh=current(host);val count=if(fresh)host.state.count else 0;val selected=selection(host)
        val busy=session?.busy==true
        return mapOf("surface" to "host_read_only","page_open" to false,"page_resumed" to false,
            "list_state" to if(!fresh)"unverified" else if(count==0)"empty" else "available",
            "row_count" to count,"strategy_selected" to (selected!=null),"busy" to busy,
            "manual_flow_active" to occupied(host),"read_enabled" to (selected!=null && !busy && !occupied(host)),
            "prepare_enabled" to false,"read_sequence" to requestCount,
            "read_outcome" to (session?.readTrace?.outcome ?: "idle"),"read_reason" to (session?.readTrace?.reason ?: "none"),
            "detail_current" to (selected!=null && session?.detailCurrent(selected)==true))
    }
    private fun release(host:BinanceHostRuntime) {
        if(hook!=null && host.onCreateObservation===hook)host.onCreateObservation=null
        hook=null;BinanceCreateSlot.shared.release(this)
    }
    private fun read(host:BinanceHostRuntime):String {
        val id=selection(host) ?: return "no_current_selection"
        if(occupied(host) || !BinanceCreateSlot.shared.acquire(this))return "manual_flow_active"
        return try {
            session?.close()
            val state=BinanceManageState(SystemClock::elapsedRealtime)
            val active=BinanceManageSession(host,state,{true}) {
                if(session?.busy==false)release(host)
            }
            session=active;requestCount++
            hook={active.observed(it)};host.onCreateObservation=hook
            active.read(id);"read_started"
        } catch(_:Exception) {runCatching{session?.close()};release(host);"read_unavailable"}
    }
    fun execute(host:BinanceHostRuntime,request:BinanceManageReadRequest):Map<String,Any?> {
        val result=when {
            request.action=="status"->"observed"
            occupied(host)->"manual_flow_active"
            session?.busy==true->"read_in_progress"
            request.action=="select"-> {
                val id=if(current(host))host.state.managementChoices().getOrNull(request.index ?: -1)?.first else null
                if(id==null)"index_unavailable" else {selectedId=id;selectedAccount=host.state.account;"selected"}
            }
            request.action=="read"->read(host)
            request.action=="reload"-> {
                if(!BinanceCreateSlot.shared.acquire(this))"manual_flow_active" else try {
                    session?.close();selectedId=null;selectedAccount=null
                    val previous=host.view
                    if(!host.begin())"login_required" else {if(host.view===previous)host.view?.reload();"reload_started"}
                } finally {BinanceCreateSlot.shared.release(this)}
            }
            else->"unsupported_action"
        }
        return facts(host)+mapOf("schema" to "yilong.binance_manage_read.v1","requested_action" to request.action,
            "result" to result,"trading_enabled" to false)
    }
}
