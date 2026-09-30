package com.elon.app.esk.compute

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import java.io.File
import org.junit.Assert.*
import org.junit.Test

class EskComputeParserTest {
    private fun fixture(): JsonObject {
        val source = generateSequence(File(System.getProperty("user.dir"))) { it.parentFile }
            .map { File(it, "contracts/esk/compute-center-v1.fixture.json") }.first { it.isFile }
        return JsonParser.parseString(source.readText()).asJsonObject
    }
    private fun parse(v: JsonObject, at: Long = 2000) = EskComputeParser.parse(v.toString().toByteArray(), at)
    @Test fun sharedContractPreservesExactMoneyAndSourceSeparation() {
        val value = parse(fixture())
        assertEquals("80000000", value.total)
        assertEquals("584000000", value.quote?.cny)
        assertEquals("80.000000", amount(value.total))
        assertEquals("12.34", amount(value.balance!!, 2))
        assertEquals(2, value.usage.size)
        assertEquals("−¥0.04 CNY", value.bills[0].amount)
        assertEquals("EskComputeSnapshot(redacted)", value.toString())
    }
    @Test fun badMoneyFreshnessOrCurrencyIsRejected() {
        val changes: List<(JsonObject) -> Unit> = listOf(
            { it.getAsJsonObject("asset").addProperty("remaining_base_units", "79000000") },
            { it.getAsJsonObject("valuation").addProperty("cny_base_units", "584000001") },
            { it.getAsJsonObject("quote").addProperty("valid_until_ms", 301001) },
            { it.getAsJsonObject("quote").addProperty("observed_at_ms", 1001) },
            { it.getAsJsonObject("billing").addProperty("currency", "ESK") },
            { it.getAsJsonObject("capabilities").addProperty("purchase", true) },
        )
        changes.forEach { change -> val v = fixture(); change(v)
            try { parse(v); fail("Expected invalid contract") } catch (_: IllegalArgumentException) { } }
        try { parse(fixture(), 61000); fail("Expected expiry") } catch (_: IllegalArgumentException) { }
    }
    @Test fun duplicateJsonKeysAndOverflowAreRejected() {
        val text = fixture().toString().replace("\"simulated\":false", "\"simulated\":false,\"simulated\":false")
        try { EskComputeParser.parse(text.toByteArray(), 2000); fail("Expected duplicate rejection") } catch (_: IllegalArgumentException) { }
        for (v in listOf("01", "1.2", "-1", "9223372036854775808")) {
            try { natural(v); fail("Expected bad amount") } catch (_: IllegalArgumentException) { }
        }
    }
}
