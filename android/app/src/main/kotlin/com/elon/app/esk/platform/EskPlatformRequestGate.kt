package com.elon.app.esk.platform

/** Pure lifecycle gate: cancellation is also enforced when a transport ignores Call.cancel(). */
internal class EskPlatformRequestGate {
    internal class Ticket internal constructor(
        internal val generation: Long,
        internal val session: EskPlatformSession,
        internal val startedElapsed: Long,
    ) {
        override fun toString() = "EskPlatformRequestTicket(redacted)"
    }

    private var generation = 0L
    private var current: Ticket? = null

    @Synchronized
    fun begin(session: EskPlatformSession, nowElapsed: Long, nowEpoch: Long, foreground: Boolean): Ticket? {
        invalidate()
        if (!foreground || nowElapsed < 0 || !session.validAt(nowEpoch)) return null
        return Ticket(generation, session, nowElapsed).also { current = it }
    }

    @Synchronized
    fun invalidate() {
        current = null
        generation = if (generation == Long.MAX_VALUE) 0 else generation + 1
    }

    @Synchronized
    fun consume(ticket: Ticket, currentSession: EskPlatformSession?, nowElapsed: Long,
        nowEpoch: Long, foreground: Boolean): Boolean {
        if (current !== ticket || generation != ticket.generation) return false
        current = null // Success and failure consume this attempt; late/duplicate callbacks cannot revive it.
        return foreground && currentSession != null && ticket.session.sameAs(currentSession) &&
            currentSession.validAt(nowEpoch) && nowElapsed >= ticket.startedElapsed &&
            nowElapsed - ticket.startedElapsed < MAX_REQUEST_MS
    }

    companion object {
        const val MAX_REQUEST_MS = 15_000L
    }
}
