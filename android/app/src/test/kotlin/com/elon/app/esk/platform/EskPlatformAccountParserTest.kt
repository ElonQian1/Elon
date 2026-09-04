package com.elon.app.esk.platform

import com.google.gson.JsonArray
import com.google.gson.JsonNull
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test

class EskPlatformAccountParserTest {
    private fun parse(json: JsonObject) = EskPlatformAccountParser.parse(json.toString().toByteArray())

    private fun rejects(json: JsonObject) {
        try {
            parse(json)
            fail("Expected rejection")
        } catch (error: IllegalArgumentException) {
            assertEquals("ESK_PLATFORM_ACCOUNT_INVALID", error.message)
            assertNull(error.cause)
        }
    }

    private fun JsonObject.firstEntry() = getAsJsonArray("entries")[0].asJsonObject

    private fun JsonObject.setAmount(amount: String, units: String) {
        addProperty("total", amount)
        addProperty("total_base_units", units)
        firstEntry().addProperty("amount", amount)
        firstEntry().addProperty("amount_base_units", units)
    }

    @Test fun validSingleRecordHasExactStringsAndNoServerMessage() {
        val json = EskPlatformAccountFixture.account()
        json.addProperty("status_message", "synthetic-message-not-for-ui")
        val account = parse(json)
        assertEquals("10.000000", account.total)
        assertEquals("10000000", account.totalBaseUnits)
        assertEquals("1", account.entryCount)
        assertEquals("2026-09-04T10:00:00+00:00", account.updatedAt)
        assertFalse(account.historyHasMore)
        assertEquals("10.000000", account.entries.single().amount)
        assertFalse(account.toString().contains("synthetic-message-not-for-ui"))
    }

    @Test fun validEmptyAccountRequiresExplicitEmptyContract() {
        val json = EskPlatformAccountFixture.account().apply {
            addProperty("total", "0.000000")
            addProperty("total_base_units", "0")
            addProperty("entry_count", "0")
            add("updated_at", JsonNull.INSTANCE)
            add("entries", JsonArray())
        }
        val account = parse(json)
        assertEquals("0.000000", account.total)
        assertTrue(account.entries.isEmpty())
        assertNull(account.updatedAt)
        json.addProperty("updated_at", "2026-09-04T10:00:00Z")
        rejects(json)
    }

    @Test fun fullHistoryConservesExactSum() {
        val json = EskPlatformAccountFixture.account().apply {
            addProperty("total", "15.000001")
            addProperty("total_base_units", "15000001")
            addProperty("entry_count", "2")
            getAsJsonArray("entries").add(EskPlatformAccountFixture.entry('b', "5.000001", "5000001",
                "2026-09-04T09:00:00+00:00"))
        }
        assertEquals(2, parse(json).entries.size)
        json.addProperty("total", "15.000002")
        json.addProperty("total_base_units", "15000002")
        rejects(json)
    }

    @Test fun truncatedHistoryUsesFullLedgerTotalAndMinimumHiddenUnits() {
        val json = EskPlatformAccountFixture.account().apply {
            addProperty("total", "10.000002")
            addProperty("total_base_units", "10000002")
            addProperty("entry_count", "3")
            addProperty("history_has_more", true)
        }
        assertTrue(parse(json).historyHasMore)
        json.addProperty("total", "10.000001")
        json.addProperty("total_base_units", "10000001")
        rejects(json)
    }

    @Test fun i64MaximumAndSingleMicroUnitKeepPrecision() {
        for ((amount, units) in listOf("9223372036854.775807" to "9223372036854775807", "0.000001" to "1")) {
            val json = EskPlatformAccountFixture.account().apply { setAmount(amount, units) }
            assertEquals(units, parse(json).totalBaseUnits)
        }
    }

    @Test fun i64MaximumCountCanOnlyDescribeSufficientPositiveUnits() {
        val json = EskPlatformAccountFixture.account().apply {
            setAmount("0.000001", "1")
            addProperty("total", "9223372036854.775807")
            addProperty("total_base_units", "9223372036854775807")
            addProperty("entry_count", "9223372036854775807")
            addProperty("history_has_more", true)
        }
        assertEquals("9223372036854775807", parse(json).entryCount)
        json.addProperty("entry_count", "9223372036854775808")
        rejects(json)
    }

    @Test fun everyRootKeyIsRequiredAndUnknownKeysAreRejected() {
        for (key in EskPlatformAccountFixture.account().keySet()) {
            rejects(EskPlatformAccountFixture.account().apply { remove(key) })
        }
        rejects(EskPlatformAccountFixture.account().apply { addProperty("user_id", "not-a-response-field") })
    }

    @Test fun everyEntryAndCapabilityKeyIsRequired() {
        for (key in EskPlatformAccountFixture.entry().keySet()) {
            rejects(EskPlatformAccountFixture.account().apply { firstEntry().remove(key) })
        }
        for (key in EskPlatformAccountFixture.account().getAsJsonObject("capabilities").keySet()) {
            rejects(EskPlatformAccountFixture.account().apply { getAsJsonObject("capabilities").remove(key) })
        }
        rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty("user_id", "x") })
        rejects(EskPlatformAccountFixture.account().apply { getAsJsonObject("capabilities").addProperty("withdraw", false) })
    }

    @Test fun everyFixedSourceAndAssetFieldMustMatch() {
        for ((key, value) in listOf("schema" to "yilong.esk.platform_account.v2", "asset_id" to "qshare",
            "symbol" to "esk", "source" to "paper_recorded", "source" to "onchain_verified",
            "chain_status" to "deployed", "verification_basis" to "chain_proof")) {
            rejects(EskPlatformAccountFixture.account().apply { addProperty(key, value) })
        }
        rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty("kind", "paper_credit") })
    }

    @Test fun allCapabilitiesAndSourceBooleansMustBeBooleanFalse() {
        for (key in listOf("simulated", "funds_moved", "external_payment_verified")) {
            rejects(EskPlatformAccountFixture.account().apply { addProperty(key, true) })
            rejects(EskPlatformAccountFixture.account().apply { addProperty(key, "false") })
        }
        for (key in EskPlatformAccountFixture.account().getAsJsonObject("capabilities").keySet()) {
            rejects(EskPlatformAccountFixture.account().apply { getAsJsonObject("capabilities").addProperty(key, true) })
            rejects(EskPlatformAccountFixture.account().apply { getAsJsonObject("capabilities").addProperty(key, "false") })
        }
    }

    @Test fun nullOrCoercedValuesDoNotMasqueradeAsStrings() {
        for (key in listOf("total", "total_base_units", "entry_count", "status_message")) {
            rejects(EskPlatformAccountFixture.account().apply { addProperty(key, 1) })
            rejects(EskPlatformAccountFixture.account().apply { add(key, JsonNull.INSTANCE) })
        }
        for (key in listOf("amount", "amount_base_units", "entry_id", "allocation_id", "created_at")) {
            rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty(key, 1) })
        }
        rejects(EskPlatformAccountFixture.account().apply { addProperty("decimals", "6") })
        rejects(EskPlatformAccountFixture.account().apply { addProperty("history_has_more", "false") })
        rejects(EskPlatformAccountFixture.account().apply { addProperty("updated_at", false) })
    }

    @Test fun noncanonicalAmountsAndOverflowAreRejected() {
        for (amount in listOf("10", "10.0", "10.0000000", "010.000000", "+10.000000", "-0.000000",
            "1e1", " 10.000000", "10.000000 ", "NaN", "Infinity", "9223372036854.775808")) {
            rejects(EskPlatformAccountFixture.account().apply { addProperty("total", amount) })
            rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty("amount", amount) })
        }
        rejects(EskPlatformAccountFixture.account().apply { setAmount("9223372036854.775808", "9223372036854775808") })
    }

    @Test fun noncanonicalBaseUnitsAndCountsAreRejected() {
        for (key in listOf("total_base_units", "entry_count")) {
            for (value in listOf("01", "-0", "-1", "+1", "1.0", "1e0", " 1", "9223372036854775808")) {
                rejects(EskPlatformAccountFixture.account().apply { addProperty(key, value) })
            }
        }
        rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty("amount_base_units", "010000000") })
    }

    @Test fun amountAndBaseUnitsMustAgreeAndEveryEntryIsPositive() {
        rejects(EskPlatformAccountFixture.account().apply { addProperty("total_base_units", "10000001") })
        rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty("amount_base_units", "10000001") })
        rejects(EskPlatformAccountFixture.account().apply { setAmount("0.000000", "0") })
    }

    @Test fun countPaginationAndEmptyHistoryMustAgree() {
        rejects(EskPlatformAccountFixture.account().apply { addProperty("entry_count", "0") })
        rejects(EskPlatformAccountFixture.account().apply { addProperty("entry_count", "2") })
        rejects(EskPlatformAccountFixture.account().apply { addProperty("history_has_more", true) })
        rejects(EskPlatformAccountFixture.account().apply { add("entries", JsonArray()) })
        rejects(EskPlatformAccountFixture.account().apply { add("updated_at", JsonNull.INSTANCE) })
    }

    @Test fun duplicateEntryOrAllocationCannotInflateHistory() {
        for (field in listOf("entry_id", "allocation_id")) {
            val json = EskPlatformAccountFixture.account().apply {
                addProperty("total", "20.000000")
                addProperty("total_base_units", "20000000")
                addProperty("entry_count", "2")
                val second = EskPlatformAccountFixture.entry('b', createdAt = "2026-09-04T09:00:00+00:00")
                second.addProperty(field, firstEntry()[field].asString)
                getAsJsonArray("entries").add(second)
            }
            rejects(json)
        }
    }

    @Test fun malformedIdentifiersAndTimestampsAreRejected() {
        for (id in listOf("", "eskp_entry_a", "eskp_entry_${"A".repeat(32)}", "user_${"a".repeat(32)}")) {
            rejects(EskPlatformAccountFixture.account().apply { firstEntry().addProperty("entry_id", id) })
        }
        for (time in listOf("", "not-a-date", "2026-02-30T10:00:00Z", "2026-09-04T24:00:00Z",
            "2026-09-04T10:00:00", "2026-09-04T10:00:00+08:00", "2026-09-04T10:00:60Z")) {
            rejects(EskPlatformAccountFixture.account().apply {
                addProperty("updated_at", time)
                firstEntry().addProperty("created_at", time)
            })
        }
    }

    @Test fun utcTimestampPrecisionMatchesProduction() {
        for (time in listOf("2026-09-04T10:00:00Z", "2026-09-04T10:00:00.123456789+00:00")) {
            val json = EskPlatformAccountFixture.account().apply {
                addProperty("updated_at", time)
                firstEntry().addProperty("created_at", time)
            }
            assertEquals(time, parse(json).updatedAt)
        }
    }

    @Test fun latestTimestampAndDescendingDatabaseOrderMustAgree() {
        rejects(EskPlatformAccountFixture.account().apply { addProperty("updated_at", "2026-09-04T09:00:00+00:00") })
        val json = EskPlatformAccountFixture.account().apply {
            addProperty("total", "20.000000")
            addProperty("total_base_units", "20000000")
            addProperty("entry_count", "2")
            getAsJsonArray("entries").add(EskPlatformAccountFixture.entry('b'))
        }
        rejects(json) // Equal-time entry b must precede entry a.
        val entries = json.getAsJsonArray("entries")
        json.add("entries", JsonArray().apply { add(entries[1]); add(entries[0]) })
        assertEquals(2, parse(json).entries.size)
    }

    @Test fun summedHistoryCannotOverflowI64() {
        val json = EskPlatformAccountFixture.account().apply {
            setAmount("9223372036854.775807", "9223372036854775807")
            addProperty("entry_count", "2")
            getAsJsonArray("entries").add(EskPlatformAccountFixture.entry('b', "0.000001", "1",
                "2026-09-04T09:00:00+00:00"))
        }
        rejects(json)
    }
}
