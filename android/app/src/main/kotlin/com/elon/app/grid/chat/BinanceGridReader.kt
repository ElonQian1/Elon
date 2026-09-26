package com.elon.app.grid.chat

import android.os.SystemClock
import com.elon.app.grid.create.BinanceCreateSlot
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.privateaccess.StrictJson

/** Main-looper confined, explicit reads only. Polling a receipt never starts a website request. */
internal object BinanceGridReader {
    private val store = BinanceGridReadStore(SystemClock::elapsedRealtime)
    private data class Job(val request: BinanceGridReadRequest, val host: BinanceHostRuntime,
        val started: Long, val owner: String, val initialAccount: String?, val baselineList: Long,
        var document: String? = null, var listDispatched: Boolean = false, var listUnavailable: Boolean = false,
        var context: BinanceGridReadStore.Context? = null, var detailBaseline: Long? = null)
    private var job: Job? = null
    fun context(host: BinanceHostRuntime): BinanceGridReadStore.Context? {
        if (!host.live() || !host.state.fresh() || host.switchingAccount || !host.adapterBound) return null
        return BinanceGridReadStore.Context(host.owner() ?: return null, host.state.account ?: return null,
            host.state.accountKind, host.document.snapshot().documentToken)
    }
    fun execute(host: BinanceHostRuntime, request: BinanceGridReadRequest): Map<String, Any?> {
        if (request.start && !store.contains(request.requestId)) start(host, request)
        return store.reply(request, context(host))
    }
    private fun start(host: BinanceHostRuntime, request: BinanceGridReadRequest) {
        store.start(request)
        if (job != null || host.onCreateObservation != null || !BinanceCreateSlot.shared.acquire(this)) {
            store.fail(request.requestId, "host_busy"); return
        }
        try {
            val initial = host.state.account.takeIf { host.live() }
            val baseline = host.state.listRevision
            if (!host.begin() || host.switchingAccount) {
                store.fail(request.requestId, "login_required"); BinanceCreateSlot.shared.release(this); return
            }
            val next = Job(request, host, SystemClock.elapsedRealtime(), host.owner() ?: error("owner_missing"), initial, baseline)
            if (initial != null && host.adapterBound) next.document = host.document.snapshot().documentToken
            job = next; tick(next)
        } catch (_: Exception) {
            store.fail(request.requestId, "host_unavailable"); job = null; BinanceCreateSlot.shared.release(this)
        }
    }
    private fun fail(active: Job, error: String) {
        store.fail(active.request.requestId, error); finish(active)
    }
    private fun finish(active: Job) {
        if (job !== active) return
        job = null; BinanceCreateSlot.shared.release(this)
    }
    private fun tick(active: Job) {
        if (job !== active) return
        try { advance(active) } catch (_: Exception) { fail(active, "read_failed") }
        if (job === active) active.host.handler.postDelayed({ tick(active) }, 250)
    }
    private fun advance(active: Job) {
        val host = active.host; val state = host.state
        if (SystemClock.elapsedRealtime() - active.started >= 45_000) return fail(active, if (active.listUnavailable) "list_context_unavailable" else "read_timeout")
        if (!host.live() || host.switchingAccount || host.owner() != active.owner) return fail(active, "context_changed")
        if (active.initialAccount != null && state.account != null && state.account != active.initialAccount) return fail(active, "context_changed")
        if (active.document != null && active.document != host.document.snapshot().documentToken) return fail(active, "context_changed")
        if (!host.adapterBound) return
        if (state.listRevision <= active.baselineList || !state.fresh()) {
            if (active.listDispatched) return
            active.listDispatched = true
            host.view?.evaluateJavascript("window.__elonBinanceReadV1?.refresh()") { result ->
                // A cold document may bind before the website issues its first authenticated list.
                if (job === active && result != "true") active.listUnavailable = true
            } ?: fail(active, "host_unavailable")
            return
        }
        val source = context(host) ?: return fail(active, "context_changed")
        if (active.document == null) active.document = source.document
        if (active.context == null) active.context = source
        if (source != active.context) return fail(active, "context_changed")
        if (active.request.kind == "list") {
            store.complete(active.request.requestId, source, state.observed, state.snapshotRows()); finish(active); return
        }
        val id = active.request.strategyId ?: return fail(active, "invalid_request")
        if (!state.contains(id)) return fail(active, "strategy_not_found")
        if (active.detailBaseline == null) {
            active.detailBaseline = state.generation
            val doc = active.document
            host.view?.evaluateJavascript("window.__elonBinanceReadV1?.detail(${StrictJson.encode(id)})") { result ->
                if (job === active && (result != "true" || doc != host.document.snapshot().documentToken)) fail(active, "detail_unavailable")
            } ?: fail(active, "host_unavailable")
            return
        }
        val observation = state.detailObservation(id) ?: return
        if (observation.first <= active.detailBaseline!!) return
        val row = state.snapshotRows().singleOrNull { it["id"] == id } ?: return fail(active, "strategy_not_found")
        store.complete(active.request.requestId, source, observation.second, listOf(row)); finish(active)
    }
}
