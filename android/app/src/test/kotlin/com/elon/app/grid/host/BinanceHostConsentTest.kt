package com.elon.app.grid.host

import org.junit.Assert.*
import org.junit.Test

class BinanceHostConsentTest {
    private val owner = "a".repeat(64)
    private val account = "b".repeat(64)
    @Test fun permissionBindsExactOwnerAccountKindAndPurpose() {
        val raw = BinanceHostConsent.encode(owner, account, "sub")
        assertTrue(BinanceHostConsent.matches(raw, owner, account, "sub"))
        assertFalse(BinanceHostConsent.matches(raw, "c".repeat(64), account, "sub"))
        assertFalse(BinanceHostConsent.matches(raw, owner, "c".repeat(64), "sub"))
        assertFalse(BinanceHostConsent.matches(raw, owner, account, "primary"))
        assertFalse(BinanceHostConsent.matches(raw.replace("grid.read", "trade"), owner, account, "sub"))
        assertFalse(BinanceHostConsent.matches(raw, owner, null, "sub"))
        assertFalse(BinanceHostConsent.matches(null, owner, account, "sub"))
    }
}
