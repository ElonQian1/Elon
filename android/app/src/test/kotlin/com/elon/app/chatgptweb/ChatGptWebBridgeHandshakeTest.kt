package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebBridgeHandshakeTest {
    @Test
    fun retriesUntilTheConfiguredAttemptLimit() {
        val scheduled = ArrayDeque<() -> Unit>()
        var injections = 0
        val handshake = ChatGptWebBridgeHandshake(
            schedule = { _, action -> scheduled.addLast(action) },
            injectAndRequestSnapshot = { injections++ },
            maxAttempts = 4,
            retryDelayMs = 1,
        )

        handshake.start()
        while (scheduled.isNotEmpty()) scheduled.removeFirst().invoke()

        assertEquals(4, injections)
    }

    @Test
    fun acknowledgementInvalidatesAlreadyScheduledRetries() {
        val scheduled = ArrayDeque<() -> Unit>()
        var injections = 0
        val handshake = ChatGptWebBridgeHandshake(
            schedule = { _, action -> scheduled.addLast(action) },
            injectAndRequestSnapshot = { injections++ },
            maxAttempts = 4,
            retryDelayMs = 1,
        )

        handshake.start()
        handshake.acknowledge()
        while (scheduled.isNotEmpty()) scheduled.removeFirst().invoke()

        assertEquals(1, injections)
    }

    @Test
    fun aNewStartInvalidatesThePreviousRetryChain() {
        val scheduled = ArrayDeque<() -> Unit>()
        var injections = 0
        val handshake = ChatGptWebBridgeHandshake(
            schedule = { _, action -> scheduled.addLast(action) },
            injectAndRequestSnapshot = { injections++ },
            maxAttempts = 2,
            retryDelayMs = 1,
        )

        handshake.start()
        handshake.start()
        while (scheduled.isNotEmpty()) scheduled.removeFirst().invoke()

        assertEquals(3, injections)
    }
}
