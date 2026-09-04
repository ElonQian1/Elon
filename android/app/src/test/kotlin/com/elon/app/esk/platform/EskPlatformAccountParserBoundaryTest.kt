package com.elon.app.esk.platform

import com.google.gson.JsonArray
import org.junit.Assert.*
import org.junit.Test

class EskPlatformAccountParserBoundaryTest {
    private fun parse(text: String) = EskPlatformAccountParser.parse(text.toByteArray(Charsets.UTF_8))

    private fun rejects(bytes: ByteArray) {
        try {
            EskPlatformAccountParser.parse(bytes)
            fail("Expected rejection")
        } catch (error: IllegalArgumentException) {
            assertEquals("ESK_PLATFORM_ACCOUNT_INVALID", error.message)
            assertNull(error.cause)
        }
    }

    private fun rejects(text: String) = rejects(text.toByteArray(Charsets.UTF_8))

    @Test fun emptyOversizedAndExactByteLimit() {
        val response = EskPlatformAccountFixture.response()
        val exact = response + " ".repeat(EskPlatformAccountParser.MAX_BYTES - response.toByteArray().size)
        assertEquals("10.000000", parse(exact).total)
        rejects(exact + " ")
        rejects(byteArrayOf())
    }

    @Test fun invalidUtf8AndBomNeverReachJsonParser() {
        val response = EskPlatformAccountFixture.response().toByteArray()
        for (invalid in listOf(byteArrayOf(0xc3.toByte(), 0x28), byteArrayOf(0xff.toByte()),
            byteArrayOf(0xed.toByte(), 0xa0.toByte(), 0x80.toByte()), byteArrayOf(0xc0.toByte(), 0xaf.toByte()))) {
            rejects(invalid + response)
        }
        rejects(byteArrayOf(0xef.toByte(), 0xbb.toByte(), 0xbf.toByte()) + response)
    }

    @Test fun duplicateRootKeysIncludingEscapedAliasesAreRejected() {
        val response = EskPlatformAccountFixture.response()
        rejects("{\"schema\":\"yilong.esk.platform_account.v1\"," + response.drop(1))
        rejects("{\"\\u0073chema\":\"yilong.esk.platform_account.v1\"," + response.drop(1))
    }

    @Test fun duplicateNestedAndEntryKeysAreRejected() {
        val response = EskPlatformAccountFixture.response()
        rejects(response.replace("\"capabilities\":{", "\"capabilities\":{\"service_spending\":false,"))
        rejects(response.replace("\"entries\":[{", "\"entries\":[{\"amount\":\"10.000000\","))
        rejects(response.replace("\"entries\":[{", "\"entries\":[{\"\\u0061mount\":\"10.000000\","))
    }

    @Test fun malformedOrLenientJsonIsRejected() {
        val response = EskPlatformAccountFixture.response()
        for (bad in listOf("/* comment */$response", "$response{}", "[$response]", "null", "true",
            response.dropLast(1) + ",}", response.replace("\"symbol\":\"ESK\"", "symbol:'ESK'"),
            response.replace("\"decimals\":6", "\"decimals\":NaN"), response.dropLast(2))) rejects(bad)
    }

    @Test fun numericDecimalsRequireExactIntegerToken() {
        val response = EskPlatformAccountFixture.response()
        for (token in listOf("6.0", "6e0", "6E+0", "06", "+6", "-6", "7", "null", "true")) {
            rejects(response.replace("\"decimals\":6", "\"decimals\":$token"))
        }
    }

    @Test fun unsafeNumbersNeverCoerceIntoFinancialStrings() {
        val response = EskPlatformAccountFixture.response()
        for (token in listOf("10000000", "9007199254740993", "9223372036854775808", "1e999")) {
            rejects(response.replace("\"total_base_units\":\"10000000\"", "\"total_base_units\":$token"))
        }
    }

    @Test fun deepOrOversizedUnknownStructuresAreBounded() {
        val response = EskPlatformAccountFixture.response()
        rejects("{\"unknown\":" + "[".repeat(20) + "0" + "]".repeat(20) + "," + response.drop(1))
        rejects("{\"unknown\":{\"${"k".repeat(65)}\":0}," + response.drop(1))
        val json = EskPlatformAccountFixture.account().apply { addProperty("status_message", "x".repeat(2049)) }
        rejects(json.toString())
    }

    @Test fun maximumOneHundredEntriesAcceptedAndNextRejected() {
        val entries = JsonArray()
        for (index in 100 downTo 1) {
            entries.add(EskPlatformAccountFixture.entry(amount = "0.000001", units = "1").apply {
                val suffix = index.toString(16).padStart(32, '0')
                addProperty("entry_id", "eskp_entry_$suffix")
                addProperty("allocation_id", "eskp_allocation_$suffix")
            })
        }
        val json = EskPlatformAccountFixture.account().apply {
            addProperty("total", "0.000100")
            addProperty("total_base_units", "100")
            addProperty("entry_count", "100")
            add("entries", entries)
        }
        assertEquals(100, parse(json.toString()).entries.size)
        entries.add(EskPlatformAccountFixture.entry('f', "0.000001", "1"))
        json.addProperty("total", "0.000101")
        json.addProperty("total_base_units", "101")
        json.addProperty("entry_count", "101")
        rejects(json.toString())
    }

    @Test fun invalidEscapesRawControlAndUnpairedSurrogatesAreRejected() {
        val response = EskPlatformAccountFixture.response()
        val original = "\"status_message\":\"Synthetic platform registration; not on chain or redeemable.\""
        for (value in listOf("bad\ncontrol", "bad\tcontrol", "\\'", "\\v", "\\uD800", "\\uDC00", "\\uQQQQ")) {
            rejects(response.replace(original, "\"status_message\":\"$value\""))
        }
    }

    @Test fun validJsonEscapesAndSurrogatePairsRemainAccepted() {
        val response = EskPlatformAccountFixture.response()
        val original = "\"status_message\":\"Synthetic platform registration; not on chain or redeemable.\""
        val escaped = response.replace(original, "\"status_message\":\"\\uD83D\\uDE00\\n\\t\\\\\\\"\\/\"")
        assertEquals("10.000000", parse(escaped).total)
        assertEquals("10.000000", parse(response.replace("\"schema\"", "\"\\u0073chema\"")).total)
    }

    @Test fun errorSurfaceNeverContainsServerInput() {
        rejects("{\"secret-synthetic-value\":\"never-echo-this\"}")
    }
}
