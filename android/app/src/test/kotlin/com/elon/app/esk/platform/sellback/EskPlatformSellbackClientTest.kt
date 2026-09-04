package com.elon.app.esk.platform.sellback

import com.google.gson.JsonParser
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Protocol
import okhttp3.Request
import okhttp3.Response
import okhttp3.ResponseBody
import okio.Buffer
import okio.BufferedSource
import okio.Timeout
import org.junit.Assert.*
import org.junit.Test
import java.io.IOException

/** Synthetic Call.Factory only. No network, DNS, account, or user money. */
class EskPlatformSellbackClientTest {
    private val f = SellbackFixture
    private val base = "https://example.invalid:9443"
    private class FakeCall(private val input: Request, private val answer: (Request) -> Response) : Call {
        var canceled = false
        private var executed = false
        override fun request() = input
        override fun execute(): Response { executed = true; return answer(input) }
        override fun enqueue(responseCallback: Callback) = error("Not used")
        override fun cancel() { canceled = true }
        override fun isExecuted() = executed
        override fun isCanceled() = canceled
        override fun timeout() = Timeout()
        override fun clone(): Call = FakeCall(input, answer)
    }
    private fun response(request: Request, code: Int = 200, type: String = "application/json", length: Long = -1,
        bytes: ByteArray = f.bytes(f.page())) = Response.Builder().request(request).protocol(Protocol.HTTP_1_1)
        .code(code).message("synthetic").body(object : ResponseBody() {
            private val buffer = Buffer().write(bytes)
            override fun contentType() = type.toMediaType()
            override fun contentLength() = length
            override fun source(): BufferedSource = buffer
        }).build()
    private fun failure(expected: SellbackNetworkFailure, block: () -> Unit) {
        val error = assertThrows(SellbackNetworkException::class.java, block)
        assertEquals(expected, error.failure); assertEquals(expected.name, error.message); assertNull(error.cause)
    }
    @Test fun allOperationsRejectHttpBeforeTokenAndTransport() {
        for (origin in listOf("http://example.invalid:8080", "https://name@host.invalid", "https://host.invalid/path",
            "https://host.invalid?x=1", "https://host.invalid/#x", "HTTPS://host.invalid", " https://host.invalid")) {
            var tokenReads = 0; var calls = 0
            fun client() = EskPlatformSellbackClient(Call.Factory { calls++; error("No call") })
            val token = { tokenReads++; "fixture-token" }
            failure(SellbackNetworkFailure.SECURE_SOURCE_REQUIRED) { client().page(origin, null, token) }
            failure(SellbackNetworkFailure.SECURE_SOURCE_REQUIRED) { client().lookup(origin, f.id(), token) }
            failure(SellbackNetworkFailure.SECURE_SOURCE_REQUIRED) { client().lookupKey(origin, "fixture-key-1", token) }
            failure(SellbackNetworkFailure.SECURE_SOURCE_REQUIRED) { client().execute(origin, f.action(), token) }
            failure(SellbackNetworkFailure.SECURE_SOURCE_REQUIRED) { client().execute(origin, SellbackAction.cancel(f.parsedRecord()), token) }
            assertEquals(0, tokenReads); assertEquals(0, calls)
        }
    }
    @Test fun listUsesOnlyFixedOriginBoundedParametersAndNoCacheCookieOrBody() {
        val client = EskPlatformSellbackClient(Call.Factory { request ->
            assertEquals("$base/api/me/assets/esk/platform/sellback-requests?limit=20", request.url.toString())
            assertEquals("GET", request.method); assertNull(request.body); assertNull(request.header("Cookie"))
            assertEquals("Bearer fixture-token", request.header("Authorization"))
            assertTrue(request.cacheControl.noCache); assertTrue(request.cacheControl.noStore)
            FakeCall(request) { response(it) }
        })
        assertEquals(100000000L, client.page(base, null) { "fixture-token" }.summary.total)
        failure(SellbackNetworkFailure.ALREADY_USED) { client.page(base, null) { error("No repeat read") } }
    }
    @Test fun invalidIdentifiersAndTokensAreRejectedBeforeCall() {
        for (value in listOf("", "bad/key", "name\nheader", "x".repeat(8192))) {
            val noCall = Call.Factory { error("No call") }
            failure(SellbackNetworkFailure.INVALID_REQUEST) { EskPlatformSellbackClient(noCall).page(base, value) { error("No token") } }
            failure(SellbackNetworkFailure.INVALID_REQUEST) { EskPlatformSellbackClient(noCall).lookup(base, value) { error("No token") } }
            failure(SellbackNetworkFailure.INVALID_REQUEST) { EskPlatformSellbackClient(noCall).lookupKey(base, value) { error("No token") } }
        }
        for (token in listOf("", "has space", "中", "x\r\nnext", "a".repeat(8193)))
            failure(SellbackNetworkFailure.SIGN_IN_REQUIRED) {
                EskPlatformSellbackClient(Call.Factory { error("No call") }).page(base, null) { token }
            }
    }
    @Test fun submitAndRetrySendExactImmutableBodyAndDoNotInventKey() {
        val bodies = mutableListOf<String>(); val action = f.action()
        repeat(2) {
            val client = EskPlatformSellbackClient(Call.Factory { request ->
                assertEquals("POST", request.method); assertNull(request.url.query)
                assertEquals("$base/api/me/assets/esk/platform/sellback-requests", request.url.toString())
                val body = Buffer().also { requireNotNull(request.body).writeTo(it) }.readUtf8(); bodies += body
                val json = JsonParser.parseString(body).asJsonObject
                assertEquals(7, json.size()); assertEquals("fixture-key-1", json["idempotency_key"].asString)
                assertEquals("SUBMIT PLATFORM ESK SELLBACK REQUEST", json["confirmation"].asString)
                FakeCall(request) { response(it, bytes = f.bytes(f.result(replayed = bodies.size > 1))) }
            })
            assertEquals("submitted", client.execute(base, action) { "fixture-token" }.request.status)
        }
        assertEquals(listOf(action.body, action.body), bodies)
    }
    @Test fun cancelAndLookupHaveDistinctBodiesAndKeysNeverEnterUrl() {
        val action = SellbackAction.cancel(f.parsedRecord())
        val cancel = EskPlatformSellbackClient(Call.Factory { request ->
            assertEquals("$base/api/me/assets/esk/platform/sellback-requests/${f.id()}/cancel", request.url.toString())
            val json = JsonParser.parseString(Buffer().also { requireNotNull(request.body).writeTo(it) }.readUtf8()).asJsonObject
            assertEquals(setOf("schema", "confirmation"), json.keySet())
            FakeCall(request) { response(it, bytes = f.bytes(f.result(true))) }
        })
        assertEquals("canceled", cancel.execute(base, action) { "fixture-token" }.request.status)
        val lookup = EskPlatformSellbackClient(Call.Factory { request ->
            assertEquals("POST", request.method); assertEquals("$base/api/me/assets/esk/platform/sellback-requests/lookup", request.url.toString())
            assertFalse(request.url.toString().contains("fixture-key"))
            val json = JsonParser.parseString(Buffer().also { requireNotNull(request.body).writeTo(it) }.readUtf8()).asJsonObject
            assertEquals(setOf("schema", "idempotency_key"), json.keySet())
            assertEquals("yilong.esk.platform_sellback_lookup.v1", json["schema"].asString)
            FakeCall(request) { response(it, bytes = f.bytes(f.result(true, true))) }
        })
        assertTrue(lookup.lookupKey(base, "fixture-key-1") { "fixture-token" }.replayed)
    }
    @Test fun responseMustMatchOriginalActionIdKeyAndCanceledReplayMeaning() {
        val invalidResponses = listOf(f.result().apply { getAsJsonObject("request").addProperty("idempotency_key", "other") },
            f.result().apply { getAsJsonObject("request").addProperty("expected_snapshot_digest", "d".repeat(64)) },
            f.result(true, false))
        for (json in invalidResponses) {
            val client = EskPlatformSellbackClient(Call.Factory { request -> FakeCall(request) { response(it, bytes = f.bytes(json)) } })
            failure(SellbackNetworkFailure.INVALID_RESPONSE) { client.execute(base, f.action()) { "fixture-token" } }
        }
        val mismatch = EskPlatformSellbackClient(Call.Factory { request -> FakeCall(request) { response(it, bytes = f.bytes(f.result())) } })
        failure(SellbackNetworkFailure.INVALID_RESPONSE) { mismatch.lookup(base, f.id(2)) { "fixture-token" } }
        val notReplay = EskPlatformSellbackClient(Call.Factory { request -> FakeCall(request) { response(it, bytes = f.bytes(f.result())) } })
        failure(SellbackNetworkFailure.INVALID_RESPONSE) { notReplay.lookupKey(base, "fixture-key-1") { "fixture-token" } }
    }
    @Test fun everyWriteTransportFailureIsSanitizedAndNeverAutomaticallyRetried() {
        for ((code, expected) in listOf(400 to SellbackNetworkFailure.INVALID_REQUEST, 401 to SellbackNetworkFailure.SIGN_IN_REQUIRED,
            403 to SellbackNetworkFailure.UNAVAILABLE, 404 to SellbackNetworkFailure.NOT_FOUND, 409 to SellbackNetworkFailure.CONFLICT,
            503 to SellbackNetworkFailure.UNAVAILABLE, 500 to SellbackNetworkFailure.NETWORK_FAILED, 307 to SellbackNetworkFailure.NETWORK_FAILED)) {
            var calls = 0
            val client = EskPlatformSellbackClient(Call.Factory { request -> calls++
                FakeCall(request) { response(it, code, bytes = "private server error".toByteArray()) }
            })
            failure(expected) { client.execute(base, f.action()) { "fixture-token" } }; assertEquals(1, calls)
        }
        val client = EskPlatformSellbackClient(Call.Factory { request -> FakeCall(request) { throw IOException("private details") } })
        failure(SellbackNetworkFailure.NETWORK_FAILED) { client.execute(base, f.action()) { "fixture-token" } }
    }
    @Test fun cancellationNeverReturnsSuccessOrReadsCanceledCredentials() {
        val early = EskPlatformSellbackClient(Call.Factory { error("No call") }); early.cancel()
        failure(SellbackNetworkFailure.CANCELED) { early.execute(base, f.action()) { error("No token") } }
        lateinit var client: EskPlatformSellbackClient
        lateinit var call: FakeCall
        client = EskPlatformSellbackClient(Call.Factory { request ->
            FakeCall(request) { client.cancel(); response(it, bytes = f.bytes(f.result())) }.also { call = it }
        })
        failure(SellbackNetworkFailure.CANCELED) { client.execute(base, f.action()) { "fixture-token" } }
        assertTrue(call.canceled)
    }
    @Test fun wrongMediaInvalidSchemaAndDeclaredOrActualOverLimitAreRejected() {
        val cases = listOf(Triple("text/html", -1L, f.bytes(f.result())),
            Triple("application/json; charset=windows-1252", -1L, f.bytes(f.result())),
            Triple("application/json", -1L, "{}".toByteArray()),
            Triple("application/json", EskPlatformSellbackParser.MAX_BYTES + 1L, f.bytes(f.result())),
            Triple("application/json", 1L, ByteArray(EskPlatformSellbackParser.MAX_BYTES + 1) { 32 }))
        for ((type, length, bytes) in cases) {
            val client = EskPlatformSellbackClient(Call.Factory { request -> FakeCall(request) { response(it, type = type, length = length, bytes = bytes) } })
            failure(SellbackNetworkFailure.INVALID_RESPONSE) { client.execute(base, f.action()) { "fixture-token" } }
        }
    }
}
