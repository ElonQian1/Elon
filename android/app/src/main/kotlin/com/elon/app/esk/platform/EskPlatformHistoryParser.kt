package com.elon.app.esk.platform

import com.elon.app.esk.platform.EskPlatformJson.NumberToken
import java.math.BigInteger
import java.time.OffsetDateTime

/** Independent history schema; shared JSON mechanics never coerce numbers into strings. */
internal object EskPlatformHistoryParser {
    const val MAX_BYTES = EskPlatformJson.MAX_BYTES
    private const val ERROR = "ESK_PLATFORM_HISTORY_INVALID"
    private val maxUnits = BigInteger.valueOf(Long.MAX_VALUE)
    private val integerPattern = Regex("0|[1-9][0-9]{0,18}")
    private val amountPattern = Regex("(0|[1-9][0-9]{0,12})\\.[0-9]{6}")
    private val digestPattern = Regex("[0-9a-f]{64}")
    private val cursorPattern = Regex("ephp1\\.([0-9a-f]{64})\\.(eskp_entry_[0-9a-f]{32})")
    private val timePattern = Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]{1,9})?(Z|\\+00:00)")
    private val rootKeys = setOf("schema", "asset_id", "symbol", "decimals", "source",
        "chain_status", "simulated", "funds_moved", "verification_basis", "external_payment_verified",
        "snapshot_digest", "total", "total_base_units", "entry_count", "range_start", "range_end",
        "updated_at", "entries", "has_more", "next_cursor")
    private val entryKeys = setOf("entry_id", "allocation_id", "amount", "amount_base_units", "created_at", "kind")

    fun validCursor(cursor: String): Boolean = cursorPattern.matches(cursor)

    fun parse(bytes: ByteArray, requestedCursor: String? = null): EskPlatformHistoryPage = try {
        require(requestedCursor == null || validCursor(requestedCursor))
        validatePage(EskPlatformJson.readObject(bytes), requestedCursor)
    } catch (_: Exception) {
        // No server text, cursor, identity or parser context is retained in failures.
        throw IllegalArgumentException(ERROR)
    }

    private fun validatePage(root: Map<String, Any?>, requestedCursor: String?): EskPlatformHistoryPage {
        require(root.keys == rootKeys && root["schema"] == "yilong.esk.platform_history.v1")
        require(root["asset_id"] == "esk" && root["symbol"] == "ESK" && root["decimals"] == NumberToken("6"))
        require(root["source"] == "platform_recorded" && root["chain_status"] == "not_deployed")
        require(root["verification_basis"] == "authenticated_operator_review")
        require(root["simulated"] == false && root["funds_moved"] == false && root["external_payment_verified"] == false)
        val digest = root.string("snapshot_digest").also { require(digestPattern.matches(it)) }
        val total = root.string("total")
        val totalRaw = root.string("total_base_units")
        val totalUnits = amountUnits(total, totalRaw)
        val countRaw = root.string("entry_count")
        val count = integer(countRaw)
        val startRaw = root.string("range_start")
        val endRaw = root.string("range_end")
        val start = integer(startRaw)
        val end = integer(endRaw)
        val updated = root.nullableString("updated_at")?.also { timestamp(it) }
        val more = root["has_more"] as? Boolean ?: error(ERROR)
        val next = root.nullableString("next_cursor")
        val entries = (root["entries"] as? List<*> ?: error(ERROR)).map { entry(it.asObject()) }
        require(entries.map { it.entryId }.toSet().size == entries.size)
        require(entries.map { it.allocationId }.toSet().size == entries.size)
        // Ordering is the SQL's original UTC text, then id, rather than normalized timestamps.
        for ((newer, older) in entries.zipWithNext()) {
            require(newer.createdAt > older.createdAt ||
                (newer.createdAt == older.createdAt && newer.entryId > older.entryId))
        }
        val size = BigInteger.valueOf(entries.size.toLong())
        val sum = entries.fold(BigInteger.ZERO) { acc, item -> acc + integer(item.amountBaseUnits) }
        if (count == BigInteger.ZERO) {
            require(requestedCursor == null && totalUnits == BigInteger.ZERO && updated == null)
            require(entries.isEmpty() && start == BigInteger.ZERO && end == BigInteger.ZERO && !more && next == null)
        } else {
            require(entries.isNotEmpty() && totalUnits > BigInteger.ZERO && count <= totalUnits)
            require(start >= BigInteger.ONE && end >= start && end <= count && end - start + BigInteger.ONE == size)
            require(updated != null && updated >= entries.first().createdAt)
            require(more == (end < count))
            if (start == BigInteger.ONE && end == count) require(totalUnits == sum)
            else require(totalUnits >= sum + count - size)
            if (requestedCursor == null) {
                require(start == BigInteger.ONE && updated == entries.first().createdAt)
            } else {
                require(start > BigInteger.ONE)
                val request = requireNotNull(cursorPattern.matchEntire(requestedCursor)).groupValues
                require(request[1] == digest && entries.none { it.entryId == request[2] })
            }
            require((next != null) == more)
            if (next != null) {
                val cursor = requireNotNull(cursorPattern.matchEntire(next)).groupValues
                require(cursor[1] == digest && cursor[2] == entries.last().entryId && next != requestedCursor)
            }
        }
        // A page cannot recompute the complete authenticated ledger digest; it binds syntax and context only.
        return EskPlatformHistoryPage(digest, total, totalRaw, countRaw, updated, startRaw, endRaw,
            entries.toList(), more, next)
    }

    private fun entry(value: Map<String, Any?>): EskPlatformEntry {
        require(value.keys == entryKeys && value["kind"] == "approved_payment_allocation")
        val id = value.string("entry_id")
        val allocation = value.string("allocation_id")
        require(Regex("eskp_entry_[0-9a-f]{32}").matches(id))
        require(Regex("eskp_allocation_[0-9a-f]{32}").matches(allocation))
        val amount = value.string("amount")
        val units = value.string("amount_base_units")
        require(amountUnits(amount, units) > BigInteger.ZERO)
        val time = value.string("created_at").also { timestamp(it) }
        return EskPlatformEntry(id, allocation, amount, units, time)
    }

    private fun amountUnits(amount: String, raw: String): BigInteger {
        require(amountPattern.matches(amount))
        val units = BigInteger(amount.replace(".", ""))
        require(units == integer(raw) && units <= maxUnits)
        return units
    }

    private fun integer(value: String): BigInteger {
        require(integerPattern.matches(value))
        return BigInteger(value).also { require(it <= maxUnits) }
    }

    private fun timestamp(value: String) {
        require(timePattern.matches(value))
        OffsetDateTime.parse(value)
    }

    private fun Map<String, Any?>.string(key: String): String = this[key] as? String ?: error(ERROR)
    private fun Map<String, Any?>.nullableString(key: String): String? = this[key]?.let { it as? String ?: error(ERROR) }
    @Suppress("UNCHECKED_CAST")
    private fun Any?.asObject(): Map<String, Any?> = this as? Map<String, Any?> ?: error(ERROR)
}
