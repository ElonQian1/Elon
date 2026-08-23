package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebProxyPrepareGateTest {
    @Test
    fun deliversTheProxyCallbackOnceAndCancelsTheTimeout() {
        var timeout: Runnable? = null
        var scheduledDelay = 0L
        var cancelled = 0
        val delivered = mutableListOf<ChatGptWebProxyStatus>()
        val gate = ChatGptWebProxyPrepareGate(
            schedule = { task, delay -> timeout = task; scheduledDelay = delay },
            cancel = { if (it === timeout) cancelled += 1 },
            timeoutMs = 750L,
            fallback = { ChatGptWebProxyStatus("手机 VPN") },
            onReady = delivered::add,
        )

        gate.start()
        gate.complete(ChatGptWebProxyStatus("系统代理"))
        timeout?.run()

        assertEquals(750L, scheduledDelay)
        assertEquals(1, cancelled)
        assertEquals(listOf("系统代理"), delivered.map(ChatGptWebProxyStatus::label))
    }

    @Test
    fun timeoutFailsOpenAndIgnoresALatePlatformCallback() {
        var timeout: Runnable? = null
        val delivered = mutableListOf<ChatGptWebProxyStatus>()
        val gate = ChatGptWebProxyPrepareGate(
            schedule = { task, _ -> timeout = task },
            cancel = {},
            timeoutMs = 750L,
            fallback = { ChatGptWebProxyStatus("手机 VPN") },
            onReady = delivered::add,
        )

        gate.start()
        timeout?.run()
        gate.complete(ChatGptWebProxyStatus("迟到的代理回调"))

        assertEquals(listOf("手机 VPN"), delivered.map(ChatGptWebProxyStatus::label))
    }
}
