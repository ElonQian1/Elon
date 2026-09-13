package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceScriptDiagnosticsTest {
    private val source = "https://bin.bnbstatic.com/static/chunks/app-ab12.js"
    private fun encoded(state: BinanceScriptDiagnostics) = StrictJson.encode(state.facts())

    @Test fun retainsActionablePublicPositionWithoutMessageOrUrlSecrets() {
        val state = BinanceScriptDiagnostics()
        state.record("Uncaught ReferenceError: process is not defined private-canary", "$source?token=private-canary#private-canary", 14)
        val result = encoded(state)
        assertTrue(result.contains("ReferenceError")); assertTrue(result.contains(source))
        assertTrue(result.contains("\"line\":14")); assertTrue(result.contains("\"capability\":\"process\""))
        assertFalse(result.contains("private-canary")); assertFalse(result.contains("is not defined"))
    }

    @Test fun rejectsCredentialsPrivatePathsSpoofedOriginsTraversalAndEncodedPaths() {
        val sources = listOf("https://bin.bnbstatic.com@evil.invalid/static/a.js", "https://evil.invalid/static/a.js",
            "http://bin.bnbstatic.com/static/a.js", "https://bin.bnbstatic.com:444/static/a.js",
            "https://www.binance.com/bapi/private/a.js", "https://www.binance.com/zh-CN/trading-bots/futures/grid/NEARUSDT",
            "https://bin.bnbstatic.com/static/../private/a.js", "https://bin.bnbstatic.com/static/%70rivate/a.js",
            "https://bin.bnbstatic.com/static/a.js/secret", "data:text/javascript,private-canary", null)
        sources.forEach {
            val state = BinanceScriptDiagnostics(); state.record("TypeError: private-canary", it, 99)
            val result = encoded(state)
            assertTrue(result.contains("\"source\":\"none\"")); assertTrue(result.contains("\"line\":0"))
            assertFalse(result.contains("private-canary"))
        }
    }

    @Test fun boundsCountsDeduplicatesAndClearsAtDocumentBoundary() {
        val state = BinanceScriptDiagnostics()
        repeat(10002) { state.record("TypeError: private-canary", source, 1) }
        assertEquals(10000, state.facts()["error_count"])
        assertEquals(1, (state.facts()["errors"] as List<*>).size)
        repeat(20) { state.record("SyntaxError", source, it + 2) }
        assertEquals(6, (state.facts()["errors"] as List<*>).size)
        state.clear()
        assertEquals(0, state.facts()["error_count"])
        assertTrue((state.facts()["errors"] as List<*>).isEmpty())
    }

    @Test fun classifiesKnownCapabilitiesWithoutLeakingArbitraryNames() {
        val state = BinanceScriptDiagnostics()
        state.record("TypeError: crypto.randomUUID is not a function", source, -8)
        state.record("ReferenceError: private-canary is not defined", source, 2000000)
        val result = encoded(state)
        assertTrue(result.contains("crypto.randomUUID")); assertFalse(result.contains("private-canary"))
        assertTrue(result.contains("\"line\":0")); assertTrue(result.contains("\"line\":1000000"))
    }

    @Test fun classifiesPolicyAndResourceErrorsWithoutTheirPrivateDetails() {
        val state = BinanceScriptDiagnostics()
        state.record("Refused due to Content Security Policy: private-canary", null, 0)
        state.record("Failed to load resource: private-canary", null, 0)
        state.record(null, null, 0)
        val result = encoded(state)
        assertTrue(result.contains("content_security_policy")); assertTrue(result.contains("resource_load"))
        assertTrue(result.contains("unknown")); assertFalse(result.contains("private-canary"))
    }
}
