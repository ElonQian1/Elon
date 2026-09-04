package com.elon.app.esk.platform

import com.google.gson.JsonArray
import com.google.gson.JsonNull
import com.google.gson.JsonObject

/** Synthetic integer-ESK allocations only; no real payment or authentication material. */
internal object EskPlatformHistoryFixture {
    const val DIGEST = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    const val TIME = "2026-09-04T10:00:00+00:00"

    fun id(index: Int): String = "eskp_entry_${index.toString(16).padStart(32, '0')}"
    fun cursor(index: Int, digest: String = DIGEST): String = "ephp1.$digest.${id(index)}"

    fun entry(index: Int, time: String = TIME): JsonObject = EskPlatformAccountFixture.entry(
        amount = "1.000000", units = "1000000", createdAt = time,
    ).apply {
        addProperty("entry_id", id(index))
        addProperty("allocation_id", "eskp_allocation_${index.toString(16).padStart(32, '0')}")
    }

    fun page(start: Int = 1, end: Int = 2, count: Int = 3): JsonObject = JsonObject().apply {
        addProperty("schema", "yilong.esk.platform_history.v1")
        addProperty("asset_id", "esk")
        addProperty("symbol", "ESK")
        addProperty("decimals", 6)
        addProperty("source", "platform_recorded")
        addProperty("chain_status", "not_deployed")
        addProperty("simulated", false)
        addProperty("funds_moved", false)
        addProperty("verification_basis", "authenticated_operator_review")
        addProperty("external_payment_verified", false)
        addProperty("snapshot_digest", DIGEST)
        addProperty("total", "$count.000000")
        addProperty("total_base_units", (count * 1000000L).toString())
        addProperty("entry_count", count.toString())
        addProperty("range_start", start.toString())
        addProperty("range_end", end.toString())
        addProperty("updated_at", TIME)
        add("entries", JsonArray().apply { for (i in start..end) add(entry(count - i + 1)) })
        addProperty("has_more", end < count)
        if (end < count) addProperty("next_cursor", cursor(count - end + 1)) else add("next_cursor", JsonNull.INSTANCE)
    }

    fun empty(): JsonObject = page().apply {
        addProperty("total", "0.000000")
        for (key in listOf("total_base_units", "entry_count", "range_start", "range_end")) addProperty(key, "0")
        add("updated_at", JsonNull.INSTANCE)
        add("entries", JsonArray())
        addProperty("has_more", false)
        add("next_cursor", JsonNull.INSTANCE)
    }
}
