package com.elon.uiruntime.view

import okhttp3.Request
import okhttp3.WebSocket
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class UiRuntimeWebSocketTest {
    @Test
    fun staleSocketCallbackCannotOwnReplacementConnection() {
        val stale = FakeWebSocket()
        val replacement = FakeWebSocket()

        assertFalse(runtimeSocketOwnsCallback(replacement, stale))
        assertTrue(runtimeSocketOwnsCallback(replacement, replacement))
    }

    private class FakeWebSocket : WebSocket {
        override fun request(): Request = Request.Builder().url("http://127.0.0.1/").build()
        override fun queueSize(): Long = 0
        override fun send(text: String): Boolean = true
        override fun send(bytes: okio.ByteString): Boolean = true
        override fun close(code: Int, reason: String?): Boolean = true
        override fun cancel() = Unit
    }
}
