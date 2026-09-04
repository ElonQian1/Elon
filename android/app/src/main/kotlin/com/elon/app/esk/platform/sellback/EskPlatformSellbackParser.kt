package com.elon.app.esk.platform.sellback

import com.elon.app.esk.platform.EskPlatformJson
import java.math.BigInteger
import java.security.MessageDigest
import java.time.OffsetDateTime

/** Only the independent formal request protocol. No tolerant Paper or account-V1 fallback. */
internal object EskPlatformSellbackParser {
    const val MAX_BYTES = EskPlatformJson.MAX_BYTES
    private const val ERROR = "ESK_PLATFORM_SELLBACK_INVALID_RESPONSE"
    private val digestPattern = Regex("[0-9a-f]{64}")
    private val idPattern = Regex("eskpsr_[0-9a-f]{32}")
    private val cursorPattern = Regex("esbr1\\.([0-9a-f]{64})\\.(eskpsr_[0-9a-f]{32})")
    private val common = setOf("schema", "asset_id", "symbol", "decimals", "source", "chain_status",
        "simulated", "funds_moved", "verification_basis", "external_payment_verified", "sellback_settlement", "summary")
    fun validId(value: String) = idPattern.matches(value)
    fun validCursor(value: String) = cursorPattern.matches(value)

    fun page(bytes: ByteArray, requestedCursor: String? = null): SellbackPage = sanitized {
        require(requestedCursor == null || validCursor(requestedCursor))
        val root = EskPlatformJson.readObject(bytes)
        envelope(root, "page", setOf("requests", "range_start", "range_end", "has_more", "next_cursor"))
        val summary = summary(root.getValue("summary").obj())
        val records = (root["requests"] as? List<*> ?: error(ERROR)).map { record(it.obj()) }
        require(records.size <= 20 && records.map { it.id }.toSet().size == records.size)
        require(records.map { it.key }.toSet().size == records.size)
        records.zipWithNext().forEach { (a, b) -> require(a.created > b.created || (a.created == b.created && a.id > b.id)) }
        val start = root.units("range_start")
        val end = root.units("range_end")
        val more = root["has_more"] as? Boolean ?: error(ERROR)
        val next = root.nullable("next_cursor")
        if (summary.count == 0L) {
            require(records.isEmpty() && start == 0L && end == 0L && !more && next == null && requestedCursor == null)
        } else {
            require(records.isNotEmpty() && start >= 1 && end >= start && end <= summary.count)
            require(end - start + 1 == records.size.toLong() && more == (end < summary.count))
            if (requestedCursor == null) require(start == 1L) else {
                val cursor = requireNotNull(cursorPattern.matchEntire(requestedCursor)).groupValues
                require(start > 1 && cursor[1] == summary.digest && records.none { it.id == cursor[2] })
            }
            require(more == (next != null))
            if (next != null) {
                val cursor = requireNotNull(cursorPattern.matchEntire(next)).groupValues
                require(cursor[1] == summary.digest && cursor[2] == records.last().id && next != requestedCursor)
            }
        }
        val open = records.filter { it.status == "submitted" }
        require(summary.openCount - open.size <= summary.count - records.size)
        val sum = open.fold(BigInteger.ZERO) { acc, item -> acc + BigInteger.valueOf(item.amount) }
        require(summary.openCount >= open.size && BigInteger.valueOf(summary.reserved) >= sum +
            BigInteger.valueOf(summary.openCount - open.size))
        if (records.size.toLong() == summary.count) {
            require(summary.openCount == open.size.toLong() && BigInteger.valueOf(summary.reserved) == sum)
        }
        SellbackPage(summary, records.toList(), start, end, next)
    }

    fun result(bytes: ByteArray): SellbackResult = sanitized {
        val root = EskPlatformJson.readObject(bytes)
        envelope(root, "result", setOf("request", "replayed"))
        val summary = summary(root.getValue("summary").obj())
        val record = record(root.getValue("request").obj())
        require(summary.count > 0)
        if (record.status == "submitted") require(summary.openCount > 0 && BigInteger.valueOf(summary.reserved) >=
            BigInteger.valueOf(record.amount) + BigInteger.valueOf(summary.openCount - 1))
        else require(summary.openCount < summary.count)
        SellbackResult(summary, record, root["replayed"] as? Boolean ?: error(ERROR))
    }

    private fun envelope(root: Map<String, Any?>, kind: String, extra: Set<String>) {
        require(root.keys == common + extra && root["schema"] == "yilong.esk.platform_sellback_${kind}.v1")
        require(root["asset_id"] == "esk" && root["symbol"] == "ESK" && root["decimals"] == EskPlatformJson.NumberToken("6"))
        require(root["source"] == "platform_recorded" && root["chain_status"] == "not_deployed")
        require(root["verification_basis"] == "authenticated_operator_review")
        for (key in listOf("simulated", "funds_moved", "external_payment_verified", "sellback_settlement")) require(root[key] == false)
    }

    private fun summary(value: Map<String, Any?>): SellbackSummary {
        require(value.keys == setOf("snapshot_digest", "total_base_units", "reserved_base_units", "available_base_units",
            "open_request_count", "request_count", "new_requests_enabled", "unavailable_reason", "policy"))
        val total = value.units("total_base_units")
        val reserved = value.units("reserved_base_units")
        val available = value.units("available_base_units")
        val open = value.units("open_request_count")
        val count = value.units("request_count")
        require(reserved <= total && available == total - reserved && open <= count && open <= reserved)
        require((open == 0L) == (reserved == 0L))
        val enabled = value["new_requests_enabled"] as? Boolean ?: error(ERROR)
        val reason = value.string("unavailable_reason")
        val policy = value["policy"]?.let { policy(it.obj()) }
        if (enabled) require(reason == "enabled" && policy != null) else {
            require(reason in setOf("disabled", "configuration_invalid", "user_not_eligible", "source_mismatch") && policy == null)
        }
        return SellbackSummary(value.digest("snapshot_digest"), total, reserved, available, open, count, enabled, reason, policy)
    }

    private fun policy(value: Map<String, Any?>): SellbackPolicy {
        require(value.keys == setOf("policy_digest", "revision", "terms_digest", "terms_text", "min_request_base_units",
            "max_request_base_units", "max_open_requests_per_user", "max_reserved_base_units_per_user", "hold_mode",
            "cancel_mode", "expiry_mode", "participation_effect", "disabled_account_recovery_text"))
        require(value["hold_mode"] == "on_submit" && value["cancel_mode"] == "owner_cancel_until_settlement")
        require(value["expiry_mode"] == "none" && value["participation_effect"] == "not_modified_by_this_feature")
        val text = boundedText(value.string("terms_text"), 2048)
        val digest = value.digest("terms_digest")
        val actual = MessageDigest.getInstance("SHA-256").digest(text.toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it.toInt() and 255) }
        require(digest == actual)
        val min = value.units("min_request_base_units")
        val max = value.units("max_request_base_units")
        val open = value.units("max_open_requests_per_user")
        val reserved = value.units("max_reserved_base_units_per_user")
        require(min > 0 && max >= min && open > 0 && reserved >= max)
        return SellbackPolicy(value.digest("policy_digest"), value.identifier("revision", 80), digest, text,
            min, max, open, reserved, boundedText(value.string("disabled_account_recovery_text"), 1024))
    }

    private fun record(value: Map<String, Any?>): SellbackRecord {
        require(value.keys == setOf("request_id", "idempotency_key", "amount_base_units", "expected_snapshot_digest",
            "request_digest", "policy_revision", "policy_digest", "terms_digest", "created_at", "canceled_at", "cancel_event_id", "status"))
        val id = value.string("request_id").also { require(validId(it)) }
        val amount = value.units("amount_base_units").also { require(it > 0) }
        val created = timestamp(value.string("created_at"))
        val canceled = value.nullable("canceled_at")?.let(::timestamp)
        val event = value.nullable("cancel_event_id")
        val status = value.string("status")
        when (status) {
            "submitted" -> require(canceled == null && event == null)
            "canceled" -> require(canceled != null && OffsetDateTime.parse(canceled) >= OffsetDateTime.parse(created) &&
                event != null && Regex("eskpsc_[0-9a-f]{32}").matches(event))
            else -> error(ERROR)
        }
        return SellbackRecord(id, value.identifier("idempotency_key", 96), amount, value.digest("expected_snapshot_digest"),
            value.digest("request_digest"), value.identifier("policy_revision", 80), value.digest("policy_digest"),
            value.digest("terms_digest"), created, canceled, event, status)
    }

    private fun boundedText(value: String, max: Int): String = value.also {
        require(it.isNotBlank() && it.toByteArray(Charsets.UTF_8).size <= max)
        require(it.none { c -> c.isISOControl() && c !in "\n\r\t" })
    }
    private fun timestamp(value: String): String = value.also {
        require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]{1,9})?(Z|\\+00:00)").matches(it))
        OffsetDateTime.parse(it)
    }
    private fun Map<String, Any?>.units(key: String): Long = string(key).also {
        require(Regex("0|[1-9][0-9]{0,18}").matches(it))
    }.toLongOrNull()?.takeIf { it >= 0 } ?: error(ERROR)
    private fun Map<String, Any?>.digest(key: String) = string(key).also { require(digestPattern.matches(it)) }
    private fun Map<String, Any?>.identifier(key: String, max: Int) = string(key).also {
        require(it.length in 1..max && Regex("[A-Za-z0-9._:-]+").matches(it))
    }
    private fun Map<String, Any?>.string(key: String): String = this[key] as? String ?: error(ERROR)
    private fun Map<String, Any?>.nullable(key: String): String? = this[key]?.let { it as? String ?: error(ERROR) }
    @Suppress("UNCHECKED_CAST") private fun Any?.obj(): Map<String, Any?> = this as? Map<String, Any?> ?: error(ERROR)
    private fun <T> sanitized(block: () -> T): T = try { block() } catch (_: Exception) { throw IllegalArgumentException(ERROR) }
}
