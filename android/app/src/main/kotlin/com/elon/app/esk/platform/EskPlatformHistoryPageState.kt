package com.elon.app.esk.platform

/** Holds one page boundary only. Neither a full-history cache nor an authentication authority. */
internal class EskPlatformHistoryPageState {
    internal class Ticket internal constructor(
        internal val session: EskPlatformSession,
        internal val previous: Position?,
    ) {
        val cursor: String? get() = previous?.cursor
        override fun toString() = "EskPlatformHistoryTicket(redacted)"
    }

    internal class Position(
        val session: EskPlatformSession, val digest: String, val total: String,
        val count: String, val updatedAt: String?, val end: Long,
        val last: EskPlatformEntry?, val cursor: String?, val shownAt: Long,
    ) {
        override fun toString() = "EskPlatformHistoryPosition(redacted)"
    }

    private var displayed: Position? = null
    private var pending: Ticket? = null

    @Synchronized fun clear() { displayed = null; pending = null }

    @Synchronized fun first(session: EskPlatformSession): Ticket {
        clear()
        return Ticket(session, null).also { pending = it }
    }

    @Synchronized fun next(
        current: EskPlatformSession?, nowElapsed: Long, nowEpoch: Long, foreground: Boolean,
    ): Ticket? {
        val position = displayed
        clear()
        if (position == null || position.cursor == null || !foreground ||
            !position.session.sameAs(current) || current?.validAt(nowEpoch) != true ||
            nowElapsed < position.shownAt || nowElapsed - position.shownAt >= MAX_DISPLAY_MS) return null
        return Ticket(position.session, position).also { pending = it }
    }

    @Synchronized fun accept(
        ticket: Ticket, page: EskPlatformHistoryPage, current: EskPlatformSession?,
        nowElapsed: Long, nowEpoch: Long, foreground: Boolean,
    ): Boolean {
        if (pending !== ticket) return false // A late request must not erase a newer one.
        clear()
        if (!foreground || nowElapsed < 0 || !ticket.session.sameAs(current) ||
            current?.validAt(nowEpoch) != true) return false
        val start = page.rangeStart.toLongOrNull() ?: return false
        val end = page.rangeEnd.toLongOrNull() ?: return false
        val previous = ticket.previous
        if (previous == null) {
            if (start != if (page.entries.isEmpty()) 0L else 1L) return false
        } else {
            val first = page.entries.firstOrNull() ?: return false
            val last = previous.last ?: return false
            if (nowElapsed < previous.shownAt || nowElapsed - previous.shownAt >= MAX_DISPLAY_MS ||
                page.snapshotDigest != previous.digest || page.totalBaseUnits != previous.total ||
                page.entryCount != previous.count || page.updatedAt != previous.updatedAt ||
                previous.end == Long.MAX_VALUE || start != previous.end + 1L ||
                first.createdAt > last.createdAt ||
                (first.createdAt == last.createdAt && first.entryId >= last.entryId)) return false
        }
        displayed = Position(ticket.session, page.snapshotDigest, page.totalBaseUnits, page.entryCount,
            page.updatedAt, end, page.entries.lastOrNull(), page.nextCursor, nowElapsed)
        return true
    }

    companion object { const val MAX_DISPLAY_MS = 60_000L }
}
