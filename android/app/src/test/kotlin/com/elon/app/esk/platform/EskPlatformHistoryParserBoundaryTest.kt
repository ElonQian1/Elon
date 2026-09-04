package com.elon.app.esk.platform

import com.google.gson.JsonArray
import org.junit.Assert.*
import org.junit.Test

class EskPlatformHistoryParserBoundaryTest {
    private fun reject(bytes: ByteArray) {
        val error = assertThrows(IllegalArgumentException::class.java) { EskPlatformHistoryParser.parse(bytes) }
        assertEquals("ESK_PLATFORM_HISTORY_INVALID", error.message)
        assertNull(error.cause)
    }

    @Test fun maximumHundredEntryPageFitsBoundedBodyAndRemainsExact() {
        val bytes = EskPlatformHistoryFixture.page(1, 100, 100).toString().toByteArray()
        assertTrue(bytes.size < EskPlatformHistoryParser.MAX_BYTES)
        val page = EskPlatformHistoryParser.parse(bytes)
        assertEquals(100, page.entries.size)
        assertEquals("100.000000", page.total)
        assertFalse(page.hasMore)
        assertEquals("100", page.rangeEnd)
    }

    @Test fun hundredAndOneEntriesAndOversizeBodyAreRejected() {
        reject(EskPlatformHistoryFixture.page(1, 101, 101).toString().toByteArray())
        val json = EskPlatformHistoryFixture.page().toString()
        val max = EskPlatformHistoryParser.MAX_BYTES
        assertEquals("3", EskPlatformHistoryParser.parse(json.padEnd(max, ' ').toByteArray()).entryCount)
        reject(json.padEnd(max + 1, ' ').toByteArray())
        reject(byteArrayOf())
    }

    @Test fun illegalUtf8BomAndUnpairedSurrogatesFailWithoutContext() {
        val json = EskPlatformHistoryFixture.page().toString()
        reject(byteArrayOf(0xc3.toByte(), 0x28))
        reject(byteArrayOf(0xed.toByte(), 0xa0.toByte(), 0x80.toByte()))
        reject(("\uFEFF" + json).toByteArray())
        reject(json.replace("platform_recorded", "\\uD800").toByteArray())
        reject(json.replace("platform_recorded", "\\uDC00").toByteArray())
    }

    @Test fun duplicateKeysRejectEvenWhenEscapedAliasHasSameValue() {
        val json = EskPlatformHistoryFixture.page().toString()
        reject(json.replace("\"symbol\":\"ESK\"", "\"symbol\":\"ESK\",\"symbol\":\"ESK\"").toByteArray())
        reject(json.replace("\"symbol\":\"ESK\"", "\"symbol\":\"ESK\",\"symbo\\u006c\":\"ESK\"").toByteArray())
        reject(json.replaceFirst("\"kind\":\"approved_payment_allocation\"",
            "\"kind\":\"approved_payment_allocation\",\"ki\\u006ed\":\"approved_payment_allocation\"").toByteArray())
    }

    @Test fun permissiveJsonExtensionsRawControlsAndTrailingValuesAreRejected() {
        val json = EskPlatformHistoryFixture.page().toString()
        for (invalid in listOf("//comment\n$json", "$json {}", json.replaceFirst("{", "{unquoted:1,"),
            json.replace("\"ESK\"", "'ESK'"), json.replace("platform_recorded", "bad\ntext"),
            json.replace("platform_recorded", "bad\\x20text"), json.replace("platform_recorded", "bad\\u000g"),
            json.replace("\"decimals\":6", "\"decimals\":NaN"),
            json.replace("\"decimals\":6", "\"decimals\":Infinity"),
            json.replace("\"decimals\":6", "\"decimals\":6e0"),
            json.replace("\"decimals\":6", "\"decimals\":06"))) reject(invalid.toByteArray())
    }

    @Test fun sharedJsonDepthArrayAndStringLimitsRemainInForce() {
        val page = EskPlatformHistoryFixture.page()
        var nested = JsonArray()
        repeat(8) { nested = JsonArray().apply { add(nested) } }
        page.add("entries", nested)
        reject(page.toString().toByteArray())
        reject(EskPlatformHistoryFixture.page().apply { addProperty("snapshot_digest", "a".repeat(2049)) }.toString().toByteArray())
        val manyKeys = (1..65).joinToString(",", "{", "}") { "\"key$it\":null" }
        reject(manyKeys.toByteArray())
        reject("{\"${"x".repeat(65)}\":null}".toByteArray())
    }

    @Test fun sharedNumberTokensPreserveExactLexemesForSchemaValidation() {
        val root = EskPlatformJson.readObject("{\"n\":6.0,\"s\":\"6\",\"b\":false}".toByteArray())
        assertEquals(EskPlatformJson.NumberToken("6.0"), root["n"])
        assertEquals("6", root["s"])
        assertEquals(false, root["b"])
    }
}
