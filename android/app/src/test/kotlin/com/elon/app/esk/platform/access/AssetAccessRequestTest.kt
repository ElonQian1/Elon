package com.elon.app.esk.platform.access

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.time.Instant

/** Real org.json parsing, including bounded credentials and exact server response bindings. */
class AssetAccessRequestTest {
    private val now = Instant.parse("2026-09-05T12:00:00Z").toEpochMilli()
    private val state = "s".repeat(32)
    private val challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"

    private fun requestJson() = JSONObject()
        .put("schema", "yilong.asset_access.android_request.v1")
        .put("state", state).put("code_challenge", challenge)

    private fun request(): AssetAccessRequest = requireNotNull(AssetAccessRequest.parse(requestJson().toString()))

    private fun result() = JSONObject()
        .put("schema", "yilong.asset_access.authorization_code.v1")
        .put("code", "aac_" + "a".repeat(64)).put("state", state)
        .put("client_id", "quant.android").put("redirect_uri", AssetAccessRequest.CALLBACK)
        .put("code_expires_at", Instant.ofEpochMilli(now + 120_000).toString())
        .put("grant_id", "aag_" + "b".repeat(32))
        .put("expires_at", Instant.ofEpochMilli(now + 900_000).toString())
        .put("scopes", JSONArray(listOf("esk.summary.read", "esk.progress.read")))

    @Test fun callerCannotChooseIdentityScopesLifetimeOrRedirect() {
        val input = request()
        val body = JSONObject(input.approvalBody())
        assertEquals("quant.android", body.getString("client_id"))
        assertEquals(AssetAccessRequest.CALLBACK, body.getString("redirect_uri"))
        assertEquals(900, body.getInt("expires_in"))
        assertTrue(body.getBoolean("explicit_consent"))
        assertEquals("授权量化只读我的资产", body.getString("confirmation"))
        assertEquals(state, body.getString("state"))
        assertEquals(challenge, body.getString("code_challenge"))
        val scopes = body.getJSONArray("scopes")
        assertEquals(setOf("esk.summary.read", "esk.progress.read"), (0 until scopes.length()).map(scopes::getString).toSet())
        for (field in listOf("user_id", "client_id", "redirect_uri", "scopes", "expires_in", "token")) {
            assertNull(AssetAccessRequest.parse(requestJson().put(field, "caller-controlled").toString()))
        }
        assertFalse(input.toString().contains(state))
        assertFalse(input.toString().contains(challenge))
    }

    @Test fun malformedUnknownAndOverlongRequestsFailClosed() {
        for (raw in listOf(null, "", "not-json", "{}", "x".repeat(1025))) {
            assertNull(AssetAccessRequest.parse(raw))
        }
        assertNull(AssetAccessRequest.parse(requestJson().put("schema", "yilong.asset_access.android_request.v0").toString()))
        for (badState in listOf("s".repeat(31), "s".repeat(129), "s".repeat(31) + "\n", "s".repeat(31) + "+")) {
            assertNull(AssetAccessRequest.parse(requestJson().put("state", badState).toString()))
        }
        for (bad in listOf("A".repeat(42), "A".repeat(44), "A".repeat(42) + "=", "B".repeat(43))) {
            assertNull(AssetAccessRequest.parse(requestJson().put("code_challenge", bad).toString()))
        }
    }

    @Test fun exactValidReplyAndShortParentLifetimeAreAccepted() {
        val request = request()
        assertTrue(request.validateResult(result().toString(), now))
        val short = result().put("expires_at", Instant.ofEpochMilli(now + 45_000).toString())
            .put("code_expires_at", Instant.ofEpochMilli(now + 45_000).toString())
        assertTrue(request.validateResult(short.toString(), now))
        assertTrue(request.validateResult(result().put("scopes", JSONArray(listOf("esk.progress.read", "esk.summary.read"))).toString(), now))
    }

    @Test fun wrongStateClientRedirectGrantCodeAndVersionCannotBecomeApproval() {
        val request = request()
        val replacements = mapOf(
            "state" to "t".repeat(32),
            "client_id" to "quant.web",
            "redirect_uri" to "com.elon.quant:/wrong-callback",
            "grant_id" to "aag_" + "b".repeat(31),
            "code" to "aat_" + "a".repeat(64),
            "schema" to "yilong.asset_access.authorization_code.v0",
        )
        for ((key, value) in replacements) assertFalse(request.validateResult(result().put(key, value).toString(), now))
        assertFalse(request.validateResult(result().put("code", "aac_" + "A".repeat(64)).toString(), now))
        assertFalse(request.validateResult(result().put("master_token", "forbidden").toString(), now))
        assertFalse(request.validateResult(result().apply { remove("grant_id") }.toString(), now))
        assertFalse(request.validateResult(result().put("state", JSONObject.NULL).toString(), now))
    }

    @Test fun broaderNarrowerDuplicateAndUnknownScopesAreRejected() {
        val request = request()
        for (scopes in listOf(
            listOf("esk.summary.read"),
            listOf("esk.summary.read", "esk.summary.read"),
            listOf("esk.summary.read", "profile.read"),
            listOf("esk.summary.read", "esk.progress.read", "profile.read"),
            listOf("esk.summary.read", "esk.sellback.write"),
        )) assertFalse(request.validateResult(result().put("scopes", JSONArray(scopes)).toString(), now))
    }

    @Test fun expiredTooLongAndInconsistentLifetimesAreRejected() {
        val request = request()
        for (expiry in listOf(now - 1, now, now + 900_001)) {
            assertFalse(request.validateResult(result().put("expires_at", Instant.ofEpochMilli(expiry).toString()).toString(), now))
        }
        for (expiry in listOf(now - 1, now, now + 120_001, now + 900_001)) {
            assertFalse(request.validateResult(result().put("code_expires_at", Instant.ofEpochMilli(expiry).toString()).toString(), now))
        }
        assertFalse(request.validateResult(result().put("code_expires_at", "not-a-timestamp").toString(), now))
        val impossible = result().put("expires_at", Instant.ofEpochMilli(now + 40_000).toString())
            .put("code_expires_at", Instant.ofEpochMilli(now + 41_000).toString())
        assertFalse(request.validateResult(impossible.toString(), now))
        assertFalse(request.validateResult("x".repeat(4097), now))
    }
}
