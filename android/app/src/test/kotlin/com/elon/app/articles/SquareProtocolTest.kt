package com.elon.app.articles

import com.elon.app.articles.square.SquareApi
import org.junit.Assert.*
import org.junit.Test

class SquareProtocolTest {
    @Test fun productionUsesTrustedHttpsAndCustomCleartextIsRejected() {
        assertEquals("https://43.139.149.158:8443", SquareApi.secureBase("http://43.139.149.158:8080"))
        assertEquals("https://example.com", SquareApi.secureBase("https://example.com/"))
        assertTrue(runCatching { SquareApi.secureBase("http://example.com") }.isFailure)
        assertTrue(runCatching { SquareApi.secureBase("https://user@example.com") }.isFailure)
    }
    @Test fun receiptLinksCannotOpenArbitraryHosts() {
        assertEquals("12345", SquareApi.postId("https://www.binance.com/en/square/post/12345"))
        assertEquals("12345", SquareApi.postId("12345"))
        assertTrue(runCatching { SquareApi.postId("https://evil.test/square/post/12345") }.isFailure)
        assertTrue(runCatching { SquareApi.postId("javascript:alert(1)") }.isFailure)
    }
}
