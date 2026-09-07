package com.elon.app.grid.access

import com.elon.app.privateaccess.*
import org.junit.Assert.*
import org.junit.Test
import java.time.Instant
import java.util.Base64

class GridAccessRequestTest {
    private val state = "s".repeat(43)
    private val challenge = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32) { 1 })
    private val now = 1_780_000_000_000L
    private fun input() = linkedMapOf<String, Any?>("schema" to "yilong.grid_access.android_request.v1",
        "purpose" to "binance_grid_read", "state" to state, "code_challenge" to challenge)
    private fun approval() = linkedMapOf<String, Any?>("schema" to "yilong.asset_access.authorization_code.v1",
        "code" to "aac_" + "a".repeat(64), "state" to state, "client_id" to "quant.android",
        "redirect_uri" to "com.elon.quant:/grid-access/callback", "code_expires_at" to Instant.ofEpochMilli(now + 120000).toString(),
        "grant_id" to "aag_" + "b".repeat(32), "expires_at" to Instant.ofEpochMilli(now + 900000).toString(),
        "scopes" to listOf("grid.snapshot.read"))
    @Test fun nativeGridRequestProducesOnlyItsExactPurposeAndScope() {
        val request = GridAccessRequest.parse(StrictJson.encode(input()))!!
        val body = StrictJson.parse(request.approvalBody())
        assertEquals(listOf("grid.snapshot.read"), body.items("scopes"))
        assertEquals("binance_grid_read", body.text("purpose"))
        assertEquals("授权量化只读我的币安网格", body.text("confirmation"))
        assertTrue(request.validateResult(StrictJson.encode(approval()), now))
    }
    @Test fun oldEskRequestsAndCallerSelectedScopesAreRejected() {
        assertNull(GridAccessRequest.parse(StrictJson.encode(input().apply { this["schema"] = "yilong.asset_access.android_request.v1" })))
        assertNull(GridAccessRequest.parse(StrictJson.encode(input().apply { this["purpose"] = "esk_read" })))
        assertNull(GridAccessRequest.parse(StrictJson.encode(input().apply { this["scopes"] = listOf("esk.summary.read") })))
    }
    @Test fun approvalRequiresExactStateCallbackScopeAndExpiry() {
        val request = GridAccessRequest.parse(StrictJson.encode(input()))!!
        for ((key, replacement) in listOf("state" to "other", "redirect_uri" to "com.elon.quant:/asset-access/callback", "client_id" to "quant.web")) {
            assertFalse(request.validateResult(StrictJson.encode(approval().apply { this[key] = replacement }), now))
        }
        assertFalse(request.validateResult(StrictJson.encode(approval().apply { this["scopes"] = listOf("esk.summary.read") }), now))
        assertFalse(request.validateResult(StrictJson.encode(approval()), now + 120000))
    }
    @Test fun duplicateInputKeysAndMalformedPkceCannotCrossNativeHandoff() {
        val raw = StrictJson.encode(input()).replace("\"purpose\":", "\"purpose\":\"binance_grid_read\",\"purpose\":")
        assertNull(GridAccessRequest.parse(raw))
        assertNull(GridAccessRequest.parse(StrictJson.encode(input().apply { this["code_challenge"] = "=".repeat(43) })))
    }
}
