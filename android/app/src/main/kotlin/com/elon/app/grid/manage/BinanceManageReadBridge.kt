package com.elon.app.grid.manage

import java.lang.ref.WeakReference

internal interface BinanceManageReadEndpoint {
    fun readFacts():Map<String,Any?>
    fun readCommand(request:BinanceManageReadRequest):String
}

/** Main-looper confined. No JS, identifiers, trade controls or strong Activity references. */
internal object BinanceManageReadBridge {
    private var target=WeakReference<BinanceManageReadEndpoint>(null)
    private var last=emptyMap<String,Any?>()
    fun bind(endpoint:BinanceManageReadEndpoint){target=WeakReference(endpoint)}
    fun unbind(endpoint:BinanceManageReadEndpoint) {
        if(target.get()!==endpoint)return
        last=endpoint.readFacts();target.clear()
    }
    fun facts():Map<String,Any?> = target.get()?.readFacts() ?: (last+mapOf(
        "page_open" to false,"page_resumed" to false,"list_state" to "unverified",
        "strategy_selected" to false,"read_enabled" to false,"prepare_enabled" to false,"detail_current" to false))
    fun execute(request:BinanceManageReadRequest):Map<String,Any?> {
        val result=if(request.action=="status") "observed" else target.get()?.readCommand(request) ?: "no_active_page"
        return facts()+mapOf("schema" to "yilong.binance_manage_read.v1","requested_action" to request.action,
            "result" to result,"trading_enabled" to false)
    }
}
