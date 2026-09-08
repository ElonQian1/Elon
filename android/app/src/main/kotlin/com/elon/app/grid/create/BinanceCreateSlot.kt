package com.elon.app.grid.create

import java.util.concurrent.atomic.AtomicReference

/** One visible creation controller may own the shared crash journal and host callbacks. */
internal class BinanceCreateSlot {
    private val owner = AtomicReference<Any?>(null)
    fun acquire(candidate: Any) = owner.compareAndSet(null,candidate)
    fun release(candidate: Any) { owner.compareAndSet(candidate,null) }
}
