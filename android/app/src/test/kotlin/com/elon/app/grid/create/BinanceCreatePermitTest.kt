package com.elon.app.grid.create

import org.junit.Assert.*
import org.junit.Test

class BinanceCreatePermitTest {
    private val operation = "a".repeat(64)
    private val account = "b".repeat(64)
    private val digest = "c".repeat(64)
    private val nonce = "d".repeat(64)
    @Test fun permissionIsExactSingleUseAndExpires() {
        var now = 100L; val p = BinanceCreatePermit { now }
        p.bind(operation, account, "doc_current", digest, nonce)
        assertTrue(p.valid(operation, account, "doc_current", digest, nonce))
        for (index in 0..4) {
            val values = mutableListOf(operation, account, "doc_current", digest, nonce)
            values[index] += "x"
            assertFalse(p.valid(values[0], values[1], values[2], values[3], values[4]))
        }
        p.consume(operation, account, "doc_current", digest, nonce)
        assertFalse(p.valid(operation, account, "doc_current", digest, nonce))
        p.bind(operation, account, "doc_current", digest, nonce)
        now += 55_000
        assertFalse(p.valid(operation, account, "doc_current", digest, nonce))
    }
    @Test fun invalidatingOrReplacingPreparationRevokesPreviousConfirmation() {
        val p = BinanceCreatePermit { 0 }
        p.bind(operation, account, "doc_current", digest, nonce)
        p.clear(); assertFalse(p.valid(operation, account, "doc_current", digest, nonce))
        p.bind(operation, account, "doc_current", digest, "e".repeat(64))
        assertFalse(p.valid(operation, account, "doc_current", digest, nonce))
    }
}
