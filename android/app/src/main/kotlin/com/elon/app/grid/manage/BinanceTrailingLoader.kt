package com.elon.app.grid.manage

import android.os.Handler
import java.util.concurrent.Future

/** One cancellable public lookup per management page. Late results cannot revive a page. */
internal class BinanceTrailingLoader(private val handler:Handler) {
    private val market=BinanceTrailingMarket()
    private var generation=0L
    private var job:Future<*>?=null
    fun cancel() {
        generation++
        job?.let { it.cancel(true);if(it is Runnable)BinanceTrailingMarket.worker.remove(it) }
        job=null
    }
    fun load(symbol:String,complete:(Result<Map<String,Any>>)->Unit) {
        cancel();val expected=generation
        try {
            job=BinanceTrailingMarket.worker.submit {
                val result=runCatching{market.read(symbol)}
                handler.post {if(expected==generation){job=null;complete(result)}}
            }
        } catch(e:RuntimeException) { complete(Result.failure(e)) }
    }
}
