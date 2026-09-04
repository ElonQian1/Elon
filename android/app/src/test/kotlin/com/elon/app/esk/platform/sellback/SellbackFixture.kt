package com.elon.app.esk.platform.sellback

import com.elon.app.esk.platform.EskPlatformSession
import com.google.gson.JsonArray
import com.google.gson.JsonNull
import com.google.gson.JsonObject
import java.security.MessageDigest

/** Pure synthetic data, never real accounts, policies, keys, or transport. */
internal object SellbackFixture {
    val digest = "a".repeat(64)
    val policyDigest = "b".repeat(64)
    const val terms = "Synthetic request terms. Not a payment."
    fun hash(text: String) = MessageDigest.getInstance("SHA-256").digest(text.toByteArray())
        .joinToString("") { "%02x".format(it.toInt() and 255) }
    fun id(n: Int = 1) = "eskpsr_" + n.toString(16).padStart(32, '0')
    fun cursor(n: Int = 1) = "esbr1.$digest.${id(n)}"
    fun policy() = JsonObject().apply {
        addProperty("policy_digest", policyDigest); addProperty("revision", "fixture-v1")
        addProperty("terms_digest", hash(terms)); addProperty("terms_text", terms)
        addProperty("min_request_base_units", "1"); addProperty("max_request_base_units", "10000000")
        addProperty("max_open_requests_per_user", "100"); addProperty("max_reserved_base_units_per_user", "10000000")
        addProperty("hold_mode", "on_submit"); addProperty("cancel_mode", "owner_cancel_until_settlement")
        addProperty("expiry_mode", "none"); addProperty("participation_effect", "not_modified_by_this_feature")
        addProperty("disabled_account_recovery_text", "Contact the synthetic operator.")
    }
    fun summary(count: Int = 1, open: Int = count) = JsonObject().apply {
        addProperty("snapshot_digest", digest); addProperty("total_base_units", "100000000")
        addProperty("reserved_base_units", (open * 1000000L).toString())
        addProperty("available_base_units", (100000000L - open * 1000000L).toString())
        addProperty("open_request_count", open.toString()); addProperty("request_count", count.toString())
        addProperty("new_requests_enabled", true); addProperty("unavailable_reason", "enabled"); add("policy", policy())
    }
    fun record(n: Int = 1, canceled: Boolean = false) = JsonObject().apply {
        addProperty("request_id", id(n)); addProperty("idempotency_key", "fixture-key-$n")
        addProperty("amount_base_units", "1000000"); addProperty("expected_snapshot_digest", digest)
        addProperty("request_digest", "c".repeat(64)); addProperty("policy_revision", "fixture-v1")
        addProperty("policy_digest", policyDigest); addProperty("terms_digest", hash(terms))
        addProperty("created_at", "2026-09-04T00:00:00Z")
        if (canceled) {
            addProperty("canceled_at", "2026-09-04T00:00:01+00:00")
            addProperty("cancel_event_id", "eskpsc_" + n.toString(16).padStart(32, '0'))
        } else { add("canceled_at", JsonNull.INSTANCE); add("cancel_event_id", JsonNull.INSTANCE) }
        addProperty("status", if (canceled) "canceled" else "submitted")
    }
    private fun common(kind: String) = JsonObject().apply {
        addProperty("schema", "yilong.esk.platform_sellback_${kind}.v1")
        addProperty("asset_id", "esk"); addProperty("symbol", "ESK"); addProperty("decimals", 6)
        addProperty("source", "platform_recorded"); addProperty("chain_status", "not_deployed")
        addProperty("verification_basis", "authenticated_operator_review")
        for (key in listOf("simulated", "funds_moved", "external_payment_verified", "sellback_settlement")) addProperty(key, false)
    }
    fun page(count: Int = 1, start: Int = 1, size: Int = minOf(20, count)) = common("page").apply {
        add("summary", summary(count)); add("requests", JsonArray().apply {
            repeat(size) { add(record(count - start + 1 - it)) }
        })
        val end = if (count == 0) 0 else start + size - 1
        addProperty("range_start", if (count == 0) "0" else start.toString()); addProperty("range_end", end.toString())
        addProperty("has_more", end < count)
        if (end < count) addProperty("next_cursor", cursor(count - end + 1)) else add("next_cursor", JsonNull.INSTANCE)
    }
    fun result(canceled: Boolean = false, replayed: Boolean = false) = common("result").apply {
        add("summary", summary(1, if (canceled) 0 else 1)); add("request", record(canceled = canceled)); addProperty("replayed", replayed)
    }
    fun bytes(value: JsonObject) = value.toString().toByteArray(Charsets.UTF_8)
    fun parsedPage() = EskPlatformSellbackParser.page(bytes(page()))
    fun parsedRecord(canceled: Boolean = false) = EskPlatformSellbackParser.result(bytes(result(canceled))).request
    fun action() = SellbackAction.submit(parsedPage().summary, 1000000, "fixture-key-1")
    fun session(token: String = "fixture-token", revision: String? = "00000000-0000-0000-0000-000000000001",
        user: String = "fixture-user", expiry: Long = 2000000L): EskPlatformSession =
        requireNotNull(EskPlatformSession.fromPreferences(mutableMapOf<String, Any>("auth_token" to token,
            "auth_user_id" to user, "auth_expires_at" to expiry).apply {
                if (revision != null) put("auth_session_revision", revision)
            }, 1000L))
}
