package com.elon.app.esk.handoff

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

/** Deterministic transport doubles: these tests never open a socket or read real credentials. */
class EskSnapshotHttpsReaderTest {
    private class FakeCall(private val input: Request, private val answer: (Request) -> Response) : Call {
        var canceled = false
        private var executed = false
        override fun request() = input
        override fun execute(): Response { executed = true; return answer(input) }
        override fun enqueue(responseCallback: Callback) = error("Asynchronous transport not used")
        override fun cancel() { canceled = true }
        override fun isExecuted() = executed
        override fun isCanceled() = canceled
        override fun timeout() = Timeout()
        override fun clone(): Call = FakeCall(input, answer)
    }

    private fun response(request: Request, code: Int = 200, type: String = "application/json",
        length: Long = -1, bytes: ByteArray = "{}".toByteArray()): Response =
        Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(code).message("test")
            .body(object : ResponseBody() {
                private val data = Buffer().write(bytes)
                override fun contentType() = type.toMediaType()
                override fun contentLength() = length
                override fun source(): BufferedSource = data
            }).build()

    @Test fun redirectIsRejectedAndRequestHasOnlyFixedReadPath() {
        var count = 0
        val reader = EskSnapshotHttpsReader(Call.Factory { request ->
            count++
            assertEquals("GET", request.method)
            assertEquals("https://example.com/api/me/assets/esk", request.url.toString())
            assertEquals("Bearer test-only-placeholder", request.header("Authorization"))
            assertNull(request.header("Cookie"))
            assertTrue(request.cacheControl.noStore)
            FakeCall(request) { response(it, code = 302).newBuilder().header("Location", "http://other.invalid").build() }
        })
        assertThrows(Exception::class.java) { reader.fetch("https://example.com") { "test-only-placeholder" } }
        assertEquals(1, count)
        // A reader is one-shot even after a failed response.
        assertThrows(Exception::class.java) { reader.fetch("https://example.com") { error("Must not read again") } }
        assertEquals(1, count)
    }

    @Test fun declaredAndActualOversizeResponsesAreRejected() {
        for (length in listOf(-1L, 1L, 16385L)) {
            val reader = EskSnapshotHttpsReader(Call.Factory { request ->
                FakeCall(request) { response(it, length = length, bytes = ByteArray(16385) { 32 }) }
            })
            assertThrows(Exception::class.java) { reader.fetch("https://example.com") { "test-only-placeholder" } }
        }
    }

    @Test fun statusAndContentTypeMustMatchWithoutCoercion() {
        for ((code, type) in listOf(401 to "application/json", 204 to "application/json",
            200 to "text/html", 200 to "application/json; charset=windows-1252")) {
            val reader = EskSnapshotHttpsReader(Call.Factory { request -> FakeCall(request) { response(it, code, type) } })
            assertThrows(Exception::class.java) { reader.fetch("https://example.com") { "test-only-placeholder" } }
        }
    }

    @Test fun cancellationCancelsRegisteredCallAndNeverReturnsAnAccount() {
        lateinit var call: FakeCall
        lateinit var reader: EskSnapshotHttpsReader
        reader = EskSnapshotHttpsReader(Call.Factory { request ->
            FakeCall(request) { reader.cancel(); response(it) }.also { call = it }
        })
        assertThrows(Exception::class.java) { reader.fetch("https://example.com") { "test-only-placeholder" } }
        assertTrue(call.canceled)
    }
}
