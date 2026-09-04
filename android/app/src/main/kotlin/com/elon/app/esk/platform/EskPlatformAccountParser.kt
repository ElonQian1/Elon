package com.elon.app.esk.platform

import com.elon.app.esk.platform.EskPlatformJson.NumberToken
import java.math.BigInteger
import java.time.OffsetDateTime

/** Fail-closed boundary for GET /api/me/assets/esk/platform, separate from Paper and IPC. */
internal object EskPlatformAccountParser {
    const val MAX_BYTES = EskPlatformJson.MAX_BYTES
    private const val ERROR = "ESK_PLATFORM_ACCOUNT_INVALID"
    private val maxUnits = BigInteger.valueOf(Long.MAX_VALUE)
    private val integerPattern = Regex("0|[1-9][0-9]{0,18}")
    private val amountPattern = Regex("(0|[1-9][0-9]{0,12})\\.[0-9]{6}")
    private val timePattern = Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]{1,9})?(Z|\\+00:00)")
    private val rootKeys = setOf("schema", "asset_id", "symbol", "decimals", "source",
        "chain_status", "simulated", "funds_moved", "verification_basis", "external_payment_verified",
        "total", "total_base_units", "entry_count", "updated_at", "history_has_more", "entries",
        "capabilities", "status_message")
    private val entryKeys = setOf("entry_id", "allocation_id", "amount", "amount_base_units", "created_at", "kind")
    private val capabilityKeys = setOf("service_spending", "quant_subscription", "sellback_settlement",
        "onchain_transfer", "chain_migration")

    fun parse(bytes: ByteArray): EskPlatformAccount = try {
        validateAccount(EskPlatformJson.readObject(bytes))
    } catch (_: Exception) {
        // Never propagate server content or parser context to UI, logs or exception causes.
        throw IllegalArgumentException(ERROR)
    }

    private fun validateAccount(root: Map<String, Any?>): EskPlatformAccount {
        require(root.keys == rootKeys)
        require(root["schema"] == "yilong.esk.platform_account.v1")
        require(root["asset_id"] == "esk" && root["symbol"] == "ESK")
        require(root["decimals"] == NumberToken("6"))
        require(root["source"] == "platform_recorded" && root["chain_status"] == "not_deployed")
        require(root["verification_basis"] == "authenticated_operator_review")
        require(root["simulated"] == false && root["funds_moved"] == false)
        require(root["external_payment_verified"] == false)
        root.string("status_message") // Intentionally discarded; explanatory UI copy is static.
        val capabilities = root["capabilities"].asObject()
        require(capabilities.keys == capabilityKeys && capabilities.values.all { it == false })
        val total = root.string("total")
        val totalRaw = root.string("total_base_units")
        val totalUnits = amountUnits(total, totalRaw)
        val countRaw = root.string("entry_count")
        val count = integer(countRaw)
        val more = root["history_has_more"] as? Boolean ?: error(ERROR)
        val updated = root["updated_at"]?.let { it as? String ?: error(ERROR) }
        if (updated != null) timestamp(updated)
        val rawEntries = root["entries"] as? List<*> ?: error(ERROR)
        val entries = rawEntries.map { entry(it.asObject()) }
        require(entries.map { it.entryId }.toSet().size == entries.size)
        require(entries.map { it.allocationId }.toSet().size == entries.size)
        val pageSize = BigInteger.valueOf(entries.size.toLong())
        require(count >= pageSize && more == (count > pageSize))
        if (count == BigInteger.ZERO) {
            require(totalUnits == BigInteger.ZERO && updated == null && entries.isEmpty() && !more)
        } else {
            require(totalUnits > BigInteger.ZERO && entries.isNotEmpty())
            require(updated == entries.first().createdAt)
        }
        val pageUnits = entries.fold(BigInteger.ZERO) { sum, entry -> sum + integer(entry.amountBaseUnits) }
        require(if (more) totalUnits >= pageUnits + count - pageSize else totalUnits == pageUnits)
        // The production SQL orders raw UTC timestamps, then ids, descending.
        for ((newer, older) in entries.zipWithNext()) {
            require(newer.createdAt > older.createdAt ||
                (newer.createdAt == older.createdAt && newer.entryId > older.entryId))
        }
        return EskPlatformAccount(total, totalRaw, countRaw, updated, more, entries.toList())
    }

    private fun entry(value: Map<String, Any?>): EskPlatformEntry {
        require(value.keys == entryKeys && value["kind"] == "approved_payment_allocation")
        val id = value.string("entry_id")
        val allocationId = value.string("allocation_id")
        require(Regex("eskp_entry_[0-9a-f]{32}").matches(id))
        require(Regex("eskp_allocation_[0-9a-f]{32}").matches(allocationId))
        val amount = value.string("amount")
        val units = value.string("amount_base_units")
        require(amountUnits(amount, units) > BigInteger.ZERO)
        val createdAt = value.string("created_at")
        timestamp(createdAt)
        return EskPlatformEntry(id, allocationId, amount, units, createdAt)
    }

    private fun amountUnits(amount: String, rawUnits: String): BigInteger {
        require(amountPattern.matches(amount))
        val units = BigInteger(amount.replace(".", ""))
        require(units == integer(rawUnits) && units <= maxUnits)
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

    @Suppress("UNCHECKED_CAST")
    private fun Any?.asObject(): Map<String, Any?> = this as? Map<String, Any?> ?: error(ERROR)
}
