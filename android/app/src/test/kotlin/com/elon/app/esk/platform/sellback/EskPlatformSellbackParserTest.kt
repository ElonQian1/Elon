package com.elon.app.esk.platform.sellback

import com.google.gson.JsonNull
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test

class EskPlatformSellbackParserTest {
    private val f = SellbackFixture
    private fun parse(json: JsonObject, cursor: String? = null) = EskPlatformSellbackParser.page(f.bytes(json), cursor)
    private fun reject(json: JsonObject, result: Boolean = false) {
        val error = assertThrows(IllegalArgumentException::class.java) {
            if (result) EskPlatformSellbackParser.result(f.bytes(json)) else parse(json)
        }
        assertEquals("ESK_PLATFORM_SELLBACK_INVALID_RESPONSE", error.message); assertNull(error.cause)
    }
    @Test fun exactAmountsNeverUseFloatingPointOrClamp() {
        assertEquals("9223372036854.775807", sellbackAmount(Long.MAX_VALUE))
        assertEquals(Long.MAX_VALUE, sellbackInput("9223372036854.775807"))
        assertEquals(1L, sellbackInput("0.000001")); assertEquals(1000000L, sellbackInput("1"))
        assertEquals(1000000L, sellbackInput("1.0"))
        for (bad in listOf("0", "0.000000", "1.0000001", "01", "1.", ".1", "1e3", "+1", "-1", " 1", "１",
            "9223372036854.775808", "99999999999999", "1\n")) assertNull(bad, sellbackInput(bad))
    }
    @Test fun emptyFirstAndContinuationKeepWholeQuotaAndBoundCursor() {
        assertEquals(0L, parse(f.page(0)).summary.count)
        val first = parse(f.page(21)); assertEquals(20, first.requests.size)
        assertEquals(21000000L, first.summary.reserved); assertEquals(f.cursor(2), first.nextCursor)
        val last = parse(f.page(21, 21, 1), first.nextCursor)
        assertEquals(21L, last.start); assertEquals(first.summary, last.summary); assertNull(last.nextCursor)
        assertThrows(IllegalArgumentException::class.java) { parse(f.page(21, 21, 1), f.cursor(1)) }
        assertThrows(IllegalArgumentException::class.java) { parse(f.page(21, 21, 1), f.cursor(2).replace('a', 'd')) }
    }
    @Test fun strictEnvelopeRejectsUnknownMissingDuplicateTypeAndWrongSource() {
        reject(f.page().apply { addProperty("token", "not-allowed") })
        reject(f.page().apply { remove("funds_moved") })
        for ((key, value) in listOf("source" to "paper", "chain_status" to "deployed", "decimals" to "6"))
            reject(f.page().apply { addProperty(key, value) })
        for (key in listOf("simulated", "funds_moved", "external_payment_verified", "sellback_settlement"))
            reject(f.page().apply { addProperty(key, true) })
        val json = f.page().toString().replaceFirst("{", "{\"asset_id\":\"esk\",")
        assertThrows(IllegalArgumentException::class.java) { EskPlatformSellbackParser.page(json.toByteArray()) }
    }
    @Test fun quotaRejectsInconsistentIntegerAndOpenRequestMath() {
        for ((key, value) in listOf("total_base_units" to "999999", "reserved_base_units" to "1000001",
            "available_base_units" to "99999999", "open_request_count" to "0", "request_count" to "0"))
            reject(f.page().apply { getAsJsonObject("summary").addProperty(key, value) })
        for (value in listOf("01", "-1", "+1", "1.0", "1e6", "9223372036854775808", "１"))
            reject(f.page().apply { getAsJsonObject("summary").addProperty("total_base_units", value) })
        reject(f.page().apply { getAsJsonObject("summary").addProperty("total_base_units", 100000000) })
        reject(f.page().apply { getAsJsonArray("requests")[0].asJsonObject.addProperty("amount_base_units", "0") })
        reject(f.page(21).apply {
            getAsJsonArray("requests")[0].asJsonObject.apply {
                addProperty("status", "canceled"); addProperty("canceled_at", "2026-09-04T00:00:01Z")
                addProperty("cancel_event_id", "eskpsc_" + "d".repeat(32))
            }
        })
    }
    @Test fun policyIsBoundToExactTextAndAllLimitsAndDisabledShape() {
        fun policyChange(key: String, value: String) = f.page().apply {
            getAsJsonObject("summary").getAsJsonObject("policy").addProperty(key, value)
        }
        reject(policyChange("terms_text", f.terms + " "))
        reject(policyChange("hold_mode", "none")); reject(policyChange("revision", "not valid"))
        reject(policyChange("max_reserved_base_units_per_user", "9999999")) // max request > reserve cap
        reject(policyChange("min_request_base_units", "0")); reject(policyChange("max_open_requests_per_user", "0"))
        reject(f.page().apply { getAsJsonObject("summary").addProperty("new_requests_enabled", false) })
        for (reason in listOf("disabled", "configuration_invalid", "user_not_eligible", "source_mismatch")) {
            val page = f.page().apply { getAsJsonObject("summary").apply {
                addProperty("new_requests_enabled", false); addProperty("unavailable_reason", reason); add("policy", JsonNull.INSTANCE)
            } }
            assertFalse(parse(page).summary.enabled); assertNull(parse(page).summary.policy)
        }
        val over = "中".repeat(683)
        reject(policyChange("terms_text", over).apply { getAsJsonObject("summary").getAsJsonObject("policy")
            .addProperty("terms_digest", f.hash(over)) })
    }
    @Test fun recordDatesAndCancellationStateMustBeConsistent() {
        assertEquals("canceled", EskPlatformSellbackParser.result(f.bytes(f.result(true, true))).request.status)
        for ((key, value) in listOf("status" to "paid", "created_at" to "2026-09-04T00:00:00+08:00",
            "created_at" to "2026-02-30T00:00:00Z", "request_id" to "eskpsr_" + "A".repeat(32),
            "idempotency_key" to "key/with/path", "policy_revision" to "", "amount_base_units" to "0"))
            reject(f.result().apply { getAsJsonObject("request").addProperty(key, value) }, true)
        reject(f.result(true).apply { getAsJsonObject("request").addProperty("canceled_at", "2026-09-03T23:59:59Z") }, true)
        reject(f.result(true).apply { getAsJsonObject("request").add("cancel_event_id", JsonNull.INSTANCE) }, true)
        reject(f.result().apply { getAsJsonObject("request").addProperty("canceled_at", "2026-09-04T00:00:01Z") }, true)
        reject(f.result(true).apply { add("summary", f.summary(1, 1)) }, true)
    }
    @Test fun pageBoundsOrderAndCursorCannotLieAboutHistory() {
        reject(f.page(21, 1, 21)); reject(f.page(2).apply { addProperty("range_start", "2") })
        reject(f.page(21).apply { addProperty("next_cursor", f.cursor(1)) })
        reject(f.page(21).apply { addProperty("has_more", false) })
        reject(f.page(2).apply { getAsJsonArray("requests").set(1, getAsJsonArray("requests")[0].deepCopy()) })
        reject(f.page(2).apply { val items = getAsJsonArray("requests"); val first = items[0]; items.set(0, items[1]); items.set(1, first) })
        reject(f.page().apply { getAsJsonArray("requests")[0].asJsonObject.addProperty("extra", false) })
    }
    @Test fun utf8SizeAndTrailingDataAreStrictAndErrorsAreSanitized() {
        for (bytes in listOf(byteArrayOf(0xc3.toByte(), 0x28), ByteArray(EskPlatformSellbackParser.MAX_BYTES + 1) { 32 },
            f.bytes(f.page()) + "{}".toByteArray(), f.bytes(f.page()) + "secret".toByteArray())) {
            val error = assertThrows(IllegalArgumentException::class.java) { EskPlatformSellbackParser.page(bytes) }
            assertEquals("ESK_PLATFORM_SELLBACK_INVALID_RESPONSE", error.message); assertNull(error.cause)
        }
    }
}
