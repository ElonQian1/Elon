package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebProxyControllerTest {
    @Test
    fun normalizesSupportedHttpProxyEndpoints() {
        assertEquals(
            "http://192.168.1.2:7890",
            ChatGptWebProxyController.normalizeEndpoint("192.168.1.2:7890"),
        )
        assertEquals(
            "http://proxy.example.com:8080",
            ChatGptWebProxyController.normalizeEndpoint("HTTP://Proxy.Example.Com:8080/"),
        )
        assertEquals(
            "http://[2001:db8::1]:7890",
            ChatGptWebProxyController.normalizeEndpoint("[2001:db8::1]:7890"),
        )
    }

    @Test
    fun rejectsCredentialsUnsupportedSchemesAndIncompleteEndpoints() {
        listOf(
            "",
            "127.0.0.1",
            "127.0.0.1:0",
            "127.0.0.1:70000",
            "https://127.0.0.1:7890",
            "socks5://127.0.0.1:7890",
            "http://user:pass@127.0.0.1:7890",
            "http://127.0.0.1:7890/path",
        ).forEach { assertNull("expected invalid proxy: $it", ChatGptWebProxyController.normalizeEndpoint(it)) }
    }
}
