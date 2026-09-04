package com.elon.eskcontract

import java.time.Instant
import java.time.OffsetDateTime
import java.time.format.DateTimeFormatter

/** Per-page integrity only; does not retain or join private rows from previous pages. */
internal object EskPlatformProgressRows {
    private val idPattern = Regex("eskpsr_[0-9a-f]{32}")
    private val utcPattern = Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]{1,9})?(Z|\\+00:00)")

    private fun utc(value: String): Instant? {
        if (!value.matches(utcPattern)) return null
        return try { OffsetDateTime.parse(value, DateTimeFormatter.ISO_OFFSET_DATE_TIME).toInstant() }
        catch (_: java.time.DateTimeException) { null }
    }

    fun valid(fields: Map<String, String>, size: Int, count: Long, open: Long, reserved: Long, cursor: String): Boolean {
        var seenOpen = 0L
        var seenReserved = 0L
        var previousTime: Instant? = null
        var previousId: String? = null
        val seen = HashSet<String>()
        val anchor = cursor.takeIf { it.isNotEmpty() }?.substring(71)
        for (index in 0 until size) {
            val prefix = "request_${index}_"
            val id = fields["${prefix}id"] ?: return false
            if (!id.matches(idPattern) || !seen.add(id) || id == anchor) return false
            val amount = EskPlatformProgressContract.units(fields["${prefix}amount"] ?: return false) ?: return false
            val base = EskPlatformProgressContract.integer(fields["${prefix}amount_base_units"] ?: return false) ?: return false
            if (base == 0L || amount != java.math.BigInteger.valueOf(base)) return false
            val created = utc(fields["${prefix}created_at"] ?: return false) ?: return false
            val prior = previousTime
            if (prior != null && (created > prior || (created == prior && id >= (previousId ?: return false)))) return false
            previousTime = created
            previousId = id
            val canceledAt = fields["${prefix}canceled_at"] ?: return false
            when (fields["${prefix}status"]) {
                "submitted" -> {
                    if (canceledAt.isNotEmpty() || base > Long.MAX_VALUE - seenReserved) return false
                    seenOpen++
                    seenReserved += base
                }
                "canceled" -> if ((utc(canceledAt) ?: return false) < created) return false
                else -> return false
            }
        }
        if (seenOpen > open || seenReserved > reserved || size.toLong() > count) return false
        val remainingOpen = open - seenOpen
        val remainingAmount = reserved - seenReserved
        if (remainingOpen > count - size || remainingAmount < remainingOpen) return false
        return remainingOpen != 0L || remainingAmount == 0L
    }
}
