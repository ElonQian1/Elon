package com.elon.app.esk.handoff

import com.elon.app.OfficialQuantApkPolicy
import okhttp3.Call
import okhttp3.CookieJar
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class EskSnapshotProviderTest {
    private fun fixture(): JSONObject = JSONObject("""
        {"schema":"yilong.esk.asset_account.v2","mode":"paper","enabled":true,
        "simulated":true,"funds_moved":false,
        "asset":{"asset_id":"esk","symbol":"ESK","name":"一龙 ESK","decimals":6,
        "issuance_mode":"paper_recorded","chain_status":"not_deployed","contract_address":null},
        "balance":{"total":"10.000000","available":"7.000000","reserved_for_sellback":"1.000000",
        "reserved_for_quant":"2.000000","reserved_total":"3.000000","total_base_units":"10000000",
        "available_base_units":"7000000","sellback_reserved_base_units":"1000000",
        "quant_reserved_base_units":"2000000","reserved_base_units":"3000000","revision":3,"updated_at":null},
        "sellback":{"application_only":true,"request_enabled":true,"settlement_enabled":false,"pricing_status":"not_defined"},
        "status_message":"Untrusted server text"}
        """)

    private fun parse(value: JSONObject) = EskSnapshotAccountParser.parse(value.toString().toByteArray(Charsets.UTF_8))
    private fun rejects(value: JSONObject) { assertThrows(Exception::class.java) { parse(value) } }
    private fun rejectsRaw(value: String) {
        assertThrows(Exception::class.java) { EskSnapshotAccountParser.parse(value.toByteArray(Charsets.UTF_8)) }
    }

    @Test fun validPaperResponseReturnsOnlyAllowlistedFields() {
        val actual = parse(fixture())
        assertEquals("10.000000", actual["total"])
        assertEquals("2.000000", actual["reserved_for_quant"])
        assertEquals("3", actual["revision"])
        assertEquals(13, actual.size)
        assertFalse(actual.containsKey("status_message"))
        assertFalse(actual.containsKey("user_id"))
    }

    @Test fun disabledAndZeroAreValidOnlyWhenExplicit() {
        val disabled = fixture().put("mode", "disabled").put("enabled", false)
        disabled.getJSONObject("sellback").put("request_enabled", false)
        assertEquals("disabled", parse(disabled)["mode"])
        val zero = fixture()
        val balance = zero.getJSONObject("balance")
        listOf("total", "available", "reserved_for_sellback", "reserved_for_quant", "reserved_total")
            .forEach { balance.put(it, "0.000000") }
        listOf("total_base_units", "available_base_units", "sellback_reserved_base_units",
            "quant_reserved_base_units", "reserved_base_units").forEach { balance.put(it, "0") }
        zero.getJSONObject("sellback").put("request_enabled", false)
        assertEquals("0.000000", parse(zero)["total"])
    }

    @Test fun rejectsCoercedFlagsAndUnknownState() {
        for (key in listOf("enabled", "simulated", "funds_moved")) {
            rejects(fixture().put(key, "false"))
            rejects(fixture().put(key, 0))
            rejects(fixture().put(key, JSONObject.NULL))
        }
        rejects(fixture().put("mode", "live"))
        rejects(fixture().put("schema", "yilong.esk.asset_account.v1"))
        rejects(fixture().put("funds_moved", true))
        rejects(fixture().put("simulated", false))
    }

    @Test fun rejectsUnknownMissingAndDuplicateFields() {
        rejects(fixture().put("token", "not-permitted"))
        rejects(fixture().apply { remove("enabled") })
        rejects(fixture().apply { getJSONObject("balance").put("extra", 0) })
        val raw = fixture().toString()
        rejectsRaw(raw.replace("\"mode\":\"paper\"", "\"mode\":\"paper\",\"mode\":\"paper\""))
        rejectsRaw(raw.replace("\"total\":\"10.000000\"", "\"total\":\"10.000000\",\"total\":\"10.000000\""))
        rejectsRaw("$raw {}")
    }

    @Test fun rejectsNonPaperIdentityAndRealTypes() {
        for ((key, value) in listOf("asset_id" to "btc", "symbol" to "esk", "issuance_mode" to "on_chain",
            "chain_status" to "confirmed", "contract_address" to "null", "decimals" to "6")) {
            rejects(fixture().apply { getJSONObject("asset").put(key, value) })
        }
        rejectsRaw(fixture().toString().replace("\"decimals\":6", "\"decimals\":6.0"))
        rejectsRaw(fixture().toString().replace("\"decimals\":6", "\"decimals\":6e0"))
        rejects(fixture().apply { getJSONObject("balance").put("revision", "3") })
        rejectsRaw(fixture().toString().replace("\"revision\":3", "\"revision\":3.0"))
        rejectsRaw(fixture().toString().replace("\"revision\":3", "\"revision\":3e0"))
        rejects(fixture().apply { getJSONObject("balance").put("revision", -1) })
        rejectsRaw(fixture().toString().replace("\"revision\":3", "\"revision\":9223372036854775808"))
    }

    @Test fun rejectsRoundingExponentOverflowAndNumericAmounts() {
        for (value in listOf<Any>(10, "10", "10.0", "1e1", "010.000000", "-1.000000",
            "10.0000000", "9223372036854.775808", JSONObject.NULL)) {
            rejects(fixture().apply { getJSONObject("balance").put("total", value) })
        }
        for (value in listOf<Any>(10000000, "010000000", "10000001", "1e7", "-10000000")) {
            rejects(fixture().apply { getJSONObject("balance").put("total_base_units", value) })
        }
    }

    @Test fun rejectsDoubleCountingEvenWhenEachAmountMatchesItsBaseUnits() {
        rejects(fixture().apply { getJSONObject("balance").put("available", "8.000000").put("available_base_units", "8000000") })
        rejects(fixture().apply { getJSONObject("balance").put("reserved_for_quant", "3.000000").put("quant_reserved_base_units", "3000000") })
        rejects(fixture().apply { getJSONObject("sellback").put("settlement_enabled", true) })
        rejects(fixture().apply { getJSONObject("sellback").put("request_enabled", "true") })
    }

    @Test fun rejectsOversizeInvalidUtf8AndArrays() {
        assertThrows(Exception::class.java) { EskSnapshotAccountParser.parse(ByteArray(16385) { 32 }) }
        assertThrows(Exception::class.java) { EskSnapshotAccountParser.parse(byteArrayOf(0xC3.toByte(), 0x28)) }
        rejectsRaw("[]")
        rejectsRaw(fixture().toString().replace("\"updated_at\":null", "\"updated_at\":[]"))
    }

    @Test fun urlIsFixedHttpsOriginOnly() {
        assertEquals("https://example.com/api/me/assets/esk", eskSnapshotEndpoint("https://example.com/").toString())
        assertEquals("https://example.com:8443/api/me/assets/esk", eskSnapshotEndpoint("https://example.com:8443").toString())
        listOf("http://example.com:8080", "//example.com", "https://user:pass@example.com", "https://example.com?x=1",
            "https://example.com#f", "https://example.com/api", "https://example.com/%2F", " https://example.com",
            "https:\\example.com", "https://example.com\\@other.com").forEach { assertNull(it, eskSnapshotEndpoint(it)) }
    }

    @Test fun httpNeverReadsTokenOrConstructsNetworkCall() {
        var tokensRead = 0
        var callsCreated = 0
        val reader = EskSnapshotHttpsReader(Call.Factory { callsCreated++; error("No network allowed") })
        assertThrows(Exception::class.java) {
            reader.fetch("http://example.com:8080") { tokensRead++; "test-only-placeholder" }
        }
        assertEquals(0, tokensRead)
        assertEquals(0, callsCreated)
    }

    @Test fun canceledReaderNeverReadsTokenOrCreatesCall() {
        var tokensRead = 0
        var callsCreated = 0
        val reader = EskSnapshotHttpsReader(Call.Factory { callsCreated++; error("No network allowed") })
        reader.cancel()
        assertThrows(Exception::class.java) {
            reader.fetch("https://example.com") { tokensRead++; "test-only-placeholder" }
        }
        assertEquals(0, tokensRead)
        assertEquals(0, callsCreated)
    }

    @Test fun secureClientHasNoSharedHooksRedirectsOrCache() {
        val client = newEskSnapshotClient()
        assertFalse(client.followRedirects)
        assertFalse(client.followSslRedirects)
        assertFalse(client.retryOnConnectionFailure)
        assertNull(client.cache)
        assertSame(CookieJar.NO_COOKIES, client.cookieJar)
        assertTrue(client.interceptors.isEmpty())
        assertTrue(client.networkInterceptors.isEmpty())
        assertEquals(15000, client.callTimeoutMillis)
    }

    @Test fun callerMustBeExactCurrentSingleSignerAndNewNativeActivity() {
        val pin = setOf(OfficialQuantApkPolicy.SIGNER_SHA256)
        fun accepted(pkg: String? = "com.elon.quant", activity: String? = ESK_QUANT_ASSETS_ACTIVITY,
            signers: Set<String> = pin, version: Long? = 3, enabled: Boolean = true, alias: String? = null) =
            acceptsEskSnapshotCaller(pkg, activity, signers, version, enabled, alias)
        assertTrue(accepted())
        assertFalse(accepted(pkg = null))
        assertFalse(accepted(pkg = "com.elon.quant.debug"))
        assertFalse(accepted(activity = "com.elon.quant.MainActivity"))
        assertFalse(accepted(signers = emptySet()))
        assertFalse(accepted(signers = pin + "another-signer"))
        assertFalse(accepted(version = 2))
        assertFalse(accepted(version = null))
        assertFalse(accepted(enabled = false))
        assertFalse(accepted(alias = ESK_QUANT_ASSETS_ACTIVITY))
    }
}
