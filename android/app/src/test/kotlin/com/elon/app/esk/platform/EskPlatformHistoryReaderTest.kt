package com.elon.app.esk.platform

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

/** Fake synchronous calls only; no DNS, sockets, real session, or production ledger. */
class EskPlatformHistoryReaderTest {
    private class FakeCall(private val input: Request, private val answer: (Request) -> Response) : Call {
        var canceled = false
        private var executed = false
        override fun request() = input
        override fun execute(): Response { executed = true; return answer(input) }
        override fun enqueue(responseCallback: Callback) = error("Async transport not used")
        override fun cancel() { canceled = true }
        override fun isExecuted() = executed
        override fun isCanceled() = canceled
        override fun timeout() = Timeout()
        override fun clone(): Call = FakeCall(input, answer)
    }

    private fun response(request: Request, code: Int = 200, type: String = "application/json",
        length: Long = -1, bytes: ByteArray = EskPlatformHistoryFixture.page().toString().toByteArray()): Response =
        Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(code).message("synthetic")
            .body(object : ResponseBody() {
                private val data = Buffer().write(bytes)
                override fun contentType() = type.toMediaType()
                override fun contentLength() = length
                override fun source(): BufferedSource = data
            }).build()

    private fun failure(expected: EskPlatformHistoryReadFailure, block: () -> Unit) {
        val error = assertThrows(EskPlatformHistoryReadException::class.java, block)
        assertEquals(expected, error.failure)
        assertEquals(expected.name, error.message)
        assertNull(error.cause)
    }

    @Test fun unsafeOriginIsRejectedBeforeCredentialsOrCalls() {
        for (base in listOf("http://example.com", "https://user@example.com", "https://example.com/path",
            "https://example.com?user=other", "https://example.com#part", " https://example.com", "HTTPS://example.com",
            "https://example.com\\path", "file:///tmp")) {
            val reader = EskPlatformHistoryReader(Call.Factory { error("No call") })
            failure(EskPlatformHistoryReadFailure.SECURE_SOURCE_REQUIRED) {
                reader.fetch(base, null) { error("No credential read") }
            }
        }
    }

    @Test fun malformedLocalCursorFailsBeforeCredentialsAndCannotInjectQuery() {
        for (cursor in listOf("", "?user=other", EskPlatformHistoryFixture.cursor(2) + "&limit=100", "x".repeat(8192))) {
            val reader = EskPlatformHistoryReader(Call.Factory { error("No call") })
            failure(EskPlatformHistoryReadFailure.INVALID_REQUEST) {
                reader.fetch("https://example.com", cursor) { error("No credential read") }
            }
        }
    }

    @Test fun firstRequestUsesFixedHistoryEndpointAndOneShotReader() {
        var count = 0
        val reader = EskPlatformHistoryReader(Call.Factory { request ->
            count++
            assertEquals("https://example.com:9443/api/me/assets/esk/platform/history?limit=20", request.url.toString())
            assertEquals("GET", request.method)
            assertEquals("Bearer fixture-token", request.header("Authorization"))
            assertEquals("application/json", request.header("Accept"))
            assertNull(request.header("Cookie"))
            assertNull(request.body)
            assertTrue(request.cacheControl.noCache && request.cacheControl.noStore)
            FakeCall(request) { response(it) }
        })
        assertEquals("3", reader.fetch("https://example.com:9443/", null) { "fixture-token" }.entryCount)
        failure(EskPlatformHistoryReadFailure.ALREADY_USED) {
            reader.fetch("https://example.com", null) { error("No second credential read") }
        }
        assertEquals(1, count)
    }

    @Test fun validCursorIsAddedOnceAndResponseIsBoundToItsDigestAndAnchor() {
        val cursor = EskPlatformHistoryFixture.cursor(2)
        val reader = EskPlatformHistoryReader(Call.Factory { request ->
            assertEquals(setOf("limit", "cursor"), request.url.queryParameterNames)
            assertEquals(listOf(cursor), request.url.queryParameterValues("cursor"))
            FakeCall(request) { response(it, bytes = EskPlatformHistoryFixture.page(3, 3, 3).toString().toByteArray()) }
        })
        assertEquals("3", reader.fetch("https://example.com", cursor) { "fixture-token" }.rangeStart)
        for (badCursor in listOf(EskPlatformHistoryFixture.cursor(1), EskPlatformHistoryFixture.cursor(2, "b".repeat(64)))) {
            val invalid = EskPlatformHistoryReader(Call.Factory { request ->
                FakeCall(request) { response(it, bytes = EskPlatformHistoryFixture.page(3, 3, 3).toString().toByteArray()) }
            })
            failure(EskPlatformHistoryReadFailure.INVALID_RESPONSE) { invalid.fetch("https://example.com", badCursor) { "fixture-token" } }
        }
    }

    @Test fun invalidCredentialAndSupplierExceptionAreSanitized() {
        for (token in listOf("", "has space", "x\nheader", "中", "x".repeat(8193))) {
            val reader = EskPlatformHistoryReader(Call.Factory { error("No call") })
            failure(EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED) { reader.fetch("https://example.com", null) { token } }
        }
        val reader = EskPlatformHistoryReader(Call.Factory { error("No call") })
        failure(EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED) {
            reader.fetch("https://example.com", null) { error("Private synthetic credential failure") }
        }
    }

    @Test fun historyChangedIsDistinctAndServerMessagesRedirectsNeverEscape() {
        for (code in listOf(301, 302, 307, 400, 401, 403, 409, 500)) {
            var count = 0
            val reader = EskPlatformHistoryReader(Call.Factory { request ->
                count++
                FakeCall(request) { response(it, code, bytes = "private synthetic server response".toByteArray())
                    .newBuilder().header("Location", "http://other.invalid").build() }
            })
            val expected = when (code) {
                401 -> EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED
                409 -> EskPlatformHistoryReadFailure.HISTORY_CHANGED
                else -> EskPlatformHistoryReadFailure.NETWORK_FAILED
            }
            failure(expected) { reader.fetch("https://example.com", null) { "fixture-token" } }
            assertEquals(1, count)
        }
    }

    @Test fun wrongContentTypeCharsetOrOldAccountSchemaIsRejected() {
        for ((type, bytes) in listOf("text/html" to "<html/>".toByteArray(),
            "application/json; charset=windows-1252" to EskPlatformHistoryFixture.page().toString().toByteArray(),
            "application/json; charset=unknown-charset" to EskPlatformHistoryFixture.page().toString().toByteArray(),
            "application/json" to EskPlatformAccountFixture.response().toByteArray())) {
            val reader = EskPlatformHistoryReader(Call.Factory { request -> FakeCall(request) { response(it, type = type, bytes = bytes) } })
            failure(EskPlatformHistoryReadFailure.INVALID_RESPONSE) { reader.fetch("https://example.com", null) { "fixture-token" } }
        }
    }

    @Test fun declaredAndActualBytesAreBoundedEvenWhenLengthIsUnknownOrDishonest() {
        val max = EskPlatformHistoryParser.MAX_BYTES
        for ((length, bytes) in listOf((max + 1L) to EskPlatformHistoryFixture.page().toString().toByteArray(),
            -1L to ByteArray(max + 1) { 32 }, 1L to ByteArray(max + 1) { 32 })) {
            val reader = EskPlatformHistoryReader(Call.Factory { request -> FakeCall(request) { response(it, length = length, bytes = bytes) } })
            failure(EskPlatformHistoryReadFailure.INVALID_RESPONSE) { reader.fetch("https://example.com", null) { "fixture-token" } }
        }
    }

    @Test fun cancellationBeforeFetchOrDuringCredentialReadCannotCreateCall() {
        val before = EskPlatformHistoryReader(Call.Factory { error("No call") })
        before.cancel()
        failure(EskPlatformHistoryReadFailure.CANCELED) { before.fetch("https://example.com", null) { error("No token read") } }
        val during = EskPlatformHistoryReader(Call.Factory { error("No call") })
        failure(EskPlatformHistoryReadFailure.CANCELED) {
            during.fetch("https://example.com", null) { during.cancel(); "fixture-token" }
        }
    }

    @Test fun registeredCallCancellationDiscardsLateSuccess() {
        lateinit var call: FakeCall
        lateinit var reader: EskPlatformHistoryReader
        reader = EskPlatformHistoryReader(Call.Factory { request ->
            FakeCall(request) { reader.cancel(); response(it) }.also { call = it }
        })
        failure(EskPlatformHistoryReadFailure.CANCELED) { reader.fetch("https://example.com", null) { "fixture-token" } }
        assertTrue(call.canceled)
    }

    @Test fun transportExceptionAndCanceledTransportFailureDoNotRetainPrivateCause() {
        val reader = EskPlatformHistoryReader(Call.Factory { request -> FakeCall(request) { throw IOException("private synthetic detail") } })
        failure(EskPlatformHistoryReadFailure.NETWORK_FAILED) { reader.fetch("https://example.com", null) { "fixture-token" } }
        lateinit var canceled: EskPlatformHistoryReader
        canceled = EskPlatformHistoryReader(Call.Factory { request ->
            FakeCall(request) { canceled.cancel(); throw IOException("private synthetic detail") }
        })
        failure(EskPlatformHistoryReadFailure.CANCELED) { canceled.fetch("https://example.com", null) { "fixture-token" } }
    }
}
