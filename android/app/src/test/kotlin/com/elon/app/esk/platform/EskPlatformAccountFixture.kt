package com.elon.app.esk.platform

import com.google.gson.JsonArray
import com.google.gson.JsonObject

/** Entirely synthetic wire fixture. Not evidence of a production account or payment. */
internal object EskPlatformAccountFixture {
    fun response(): String = account().toString()

    fun account(): JsonObject = JsonObject().apply {
        addProperty("schema", "yilong.esk.platform_account.v1")
        addProperty("asset_id", "esk")
        addProperty("symbol", "ESK")
        addProperty("decimals", 6)
        addProperty("source", "platform_recorded")
        addProperty("chain_status", "not_deployed")
        addProperty("simulated", false)
        addProperty("funds_moved", false)
        addProperty("verification_basis", "authenticated_operator_review")
        addProperty("external_payment_verified", false)
        addProperty("total", "10.000000")
        addProperty("total_base_units", "10000000")
        addProperty("entry_count", "1")
        addProperty("updated_at", "2026-09-04T10:00:00+00:00")
        addProperty("history_has_more", false)
        add("entries", JsonArray().apply { add(entry()) })
        add("capabilities", JsonObject().apply {
            for (key in listOf("service_spending", "quant_subscription", "sellback_settlement",
                "onchain_transfer", "chain_migration")) addProperty(key, false)
        })
        addProperty("status_message", "Synthetic platform registration; not on chain or redeemable.")
    }

    fun entry(
        suffix: Char = 'a',
        amount: String = "10.000000",
        units: String = "10000000",
        createdAt: String = "2026-09-04T10:00:00+00:00",
    ): JsonObject = JsonObject().apply {
        addProperty("entry_id", "eskp_entry_${suffix.toString().repeat(32)}")
        addProperty("allocation_id", "eskp_allocation_${suffix.toString().repeat(32)}")
        addProperty("amount", amount)
        addProperty("amount_base_units", units)
        addProperty("created_at", createdAt)
        addProperty("kind", "approved_payment_allocation")
    }
}
