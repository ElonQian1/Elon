package com.elon.app.esk.platform

import okhttp3.Call
import okhttp3.Callback
import okhttp3.CookieJar
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
import java.net.Proxy

/** Synthetic transport only: no credentials, DNS, sockets, or server accounts. */
class EskPlatformAccountReaderTest {
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
        length: Long = -1, bytes: ByteArray = EskPlatformAccountFixture.response().toByteArray()): Response =
        Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(code).message("synthetic")
            .body(object : ResponseBody() {
                private val data = Buffer().write(bytes)
                override fun contentType() = type.toMediaType()
                override fun contentLength() = length
                override fun source(): BufferedSource = data
            }).build()

    private fun failure(expected: EskPlatformReadFailure, block: () -> Unit) {
        val error = assertThrows(EskPlatformReadException::class.java, block)
        assertEquals(expected, error.failure)
        assertEquals(expected.name, error.message)
        assertNull(error.cause)
    }

    @Test fun insecureOrAmbiguousOriginNeverReadsTokenOrCreatesCall() {
        for (base in listOf("http://example.com:8080", "https://a@host.invalid", "https://host.invalid/base",
            "https://host.invalid?x=1", "https://host.invalid/#x", " https://host.invalid",
            "https://host.invalid\\path", "file:///tmp", "https://", "HTTPS://host.invalid")) {
            var reads = 0
            var calls = 0
            val reader = EskPlatformAccountReader(Call.Factory { calls++; error("Must not make a call") })
            failure(EskPlatformReadFailure.SECURE_SOURCE_REQUIRED) { reader.fetch(base) { reads++; "fixture-token" } }
            assertEquals(base, 0, reads)
            assertEquals(base, 0, calls)
        }
    }

    @Test fun validReadUsesOnlyFixedEndpointAndIsOneShot() {
        var calls = 0
        val reader = EskPlatformAccountReader(Call.Factory { request ->
            calls++
            assertEquals("https://example.com:9443/api/me/assets/esk/platform?limit=20", request.url.toString())
            assertEquals("GET", request.method)
            assertEquals("Bearer fixture-token", request.header("Authorization"))
            assertEquals("application/json", request.header("Accept"))
            assertNull(request.header("Cookie"))
            assertNull(request.body)
            assertTrue(request.cacheControl.noCache)
            assertTrue(request.cacheControl.noStore)
            FakeCall(request) { response(it) }
        })
        assertEquals("10.000000", reader.fetch("https://example.com:9443/") { "fixture-token" }.total)
        failure(EskPlatformReadFailure.ALREADY_USED) { reader.fetch("https://example.com") { error("No second token read") } }
        assertEquals(1, calls)
    }

    @Test fun invalidTokenOrSupplierFailureNeverCreatesCallAndIsSanitized() {
        for (token in listOf("", "has space", "x\nheader", "中", "a".repeat(8193))) {
            val reader = EskPlatformAccountReader(Call.Factory { error("No call expected") })
            failure(EskPlatformReadFailure.SIGN_IN_REQUIRED) { reader.fetch("https://example.com") { token } }
        }
        val reader = EskPlatformAccountReader(Call.Factory { error("No call expected") })
        failure(EskPlatformReadFailure.SIGN_IN_REQUIRED) {
            reader.fetch("https://example.com") { error("secret fixture should not escape") }
        }
    }

    @Test fun redirectsAndServerErrorsAreFixedFailuresWithoutReadingServerMessage() {
        for (code in listOf(301, 302, 307, 401, 403, 404, 500)) {
            var calls = 0
            val reader = EskPlatformAccountReader(Call.Factory { request ->
                calls++
                FakeCall(request) { response(it, code, bytes = "private server message".toByteArray())
                    .newBuilder().header("Location", "http://other.invalid/").build() }
            })
            failure(if (code == 401) EskPlatformReadFailure.SIGN_IN_REQUIRED else EskPlatformReadFailure.NETWORK_FAILED) {
                reader.fetch("https://example.com") { "fixture-token" }
            }
            assertEquals(1, calls)
        }
    }

    @Test fun invalidContentAndInvalidSchemaAreRejected() {
        for ((type, bytes) in listOf("text/html" to "<html>no</html>".toByteArray(),
            "application/json; charset=windows-1252" to EskPlatformAccountFixture.response().toByteArray(),
            "application/json; charset=unknown-charset" to EskPlatformAccountFixture.response().toByteArray(),
            "application/json" to "{}".toByteArray())) {
            val reader = EskPlatformAccountReader(Call.Factory { request -> FakeCall(request) { response(it, type = type, bytes = bytes) } })
            failure(EskPlatformReadFailure.INVALID_RESPONSE) { reader.fetch("https://example.com") { "fixture-token" } }
        }
    }

    @Test fun declaredOrActualOversizeResponseIsRejected() {
        val maximum = EskPlatformAccountParser.MAX_BYTES
        for ((length, bytes) in listOf((maximum + 1L) to EskPlatformAccountFixture.response().toByteArray(),
            -1L to ByteArray(maximum + 1) { 32 }, 1L to ByteArray(maximum + 1) { 32 })) {
            val reader = EskPlatformAccountReader(Call.Factory { request -> FakeCall(request) { response(it, length = length, bytes = bytes) } })
            failure(EskPlatformReadFailure.INVALID_RESPONSE) { reader.fetch("https://example.com") { "fixture-token" } }
        }
    }

    @Test fun cancellationBeforeFetchSkipsCredentialReadAndCall() {
        val reader = EskPlatformAccountReader(Call.Factory { error("No call expected") })
        reader.cancel()
        failure(EskPlatformReadFailure.CANCELED) { reader.fetch("https://example.com") { error("No token read") } }
    }

    @Test fun cancellationOfRegisteredCallRejectsEvenAnOtherwiseSuccessfulResponse() {
        lateinit var call: FakeCall
        lateinit var reader: EskPlatformAccountReader
        reader = EskPlatformAccountReader(Call.Factory { request ->
            FakeCall(request) { reader.cancel(); response(it) }.also { call = it }
        })
        failure(EskPlatformReadFailure.CANCELED) { reader.fetch("https://example.com") { "fixture-token" } }
        assertTrue(call.canceled)
    }

    @Test fun clientHasNoRedirectCacheCookieProxyOrInterceptors() {
        val client = newEskPlatformClient()
        assertFalse(client.followRedirects)
        assertFalse(client.followSslRedirects)
        assertFalse(client.retryOnConnectionFailure)
        assertNull(client.cache)
        assertSame(CookieJar.NO_COOKIES, client.cookieJar)
        assertSame(Proxy.NO_PROXY, client.proxy)
        assertTrue(client.interceptors.isEmpty())
        assertTrue(client.networkInterceptors.isEmpty())
        assertEquals(15000, client.callTimeoutMillis)
        assertEquals(15000, client.connectTimeoutMillis)
        assertEquals(15000, client.readTimeoutMillis)
        assertEquals(15000, client.writeTimeoutMillis)
    }
}
