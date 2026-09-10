package com.elon.app.grid.host

import com.elon.app.grid.create.BinanceCreateSlot

/** One document reload per stalled read episode; never a private request or command replay. */
internal class BinanceReadRecovery(
    private val clock: () -> Long,
    private val slot: BinanceCreateSlot = BinanceCreateSlot.shared,
) {
    private var pageStartedAt: Long? = null
    var reloadUsed = false; private set
    var reloadCount = 0; private set

    fun pageStarted() { pageStartedAt = clock() }
    fun verifiedList() { reloadUsed = false }
    fun reset() { pageStartedAt = null; reloadUsed = false; reloadCount = 0 }

    fun recover(refreshResult: String?, canReload: () -> Boolean, reload: () -> Unit): Boolean {
        val started = pageStartedAt ?: return false
        if (refreshResult != "false" || reloadUsed || clock() - started < 10_000) return false
        if (!slot.acquire(this)) return false
        try {
            // Evaluate current document/session/UI state after acquiring the shared write slot.
            if (!canReload()) return false
            reloadUsed = true
            reloadCount++
            reload()
            return true
        } finally { slot.release(this) }
    }
}
