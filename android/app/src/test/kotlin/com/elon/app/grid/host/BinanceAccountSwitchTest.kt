package com.elon.app.grid.host

import org.junit.Assert.*
import org.junit.Test

class BinanceAccountSwitchTest {
    @Test fun switchingDoesNotAcceptMissingIdentityOrTheOldAccount() {
        val old = BinanceAccountSwitch.key("abc", "primary")
        assertFalse(BinanceAccountSwitch.accepts(old, null, "unknown"))
        assertFalse(BinanceAccountSwitch.accepts(old, "abc", "primary"))
        assertTrue(BinanceAccountSwitch.accepts(old, "def", "primary"))
        assertFalse(BinanceAccountSwitch.accepts(old, "abc", "sub"))
        assertFalse(BinanceAccountSwitch.accepts(old, "def", "invalid"))
    }
    @Test fun summaryDoesNotInventAnAccountOrExposeTheFullFingerprint() {
        assertTrue(BinanceAccountSwitch.label(null, "unknown").contains("尚未确认"))
        val summary = BinanceAccountSwitch.label("123456789abcdef", "sub")
        assertTrue(summary.contains("子账户"))
        assertTrue(summary.contains("12345678"))
        assertFalse(summary.contains("123456789abcdef"))
    }
    @Test fun previousGrantNeverRevivesAfterChangingAccountOrReturningToIt() {
        val state = BinanceHostState({ 1000L }, { 2000L })
        fun observe(uid: String) = state.accept(
            "{\"kind\":\"list\",\"account\":\"$uid\",\"account_kind\":\"primary\",\"rows\":[]}")
        observe("42")
        val old = state.grant(continuous = true)
        val previous = BinanceAccountSwitch.key(state.account!!, state.accountKind)
        state.unavailable()
        observe("42")
        assertFalse(BinanceAccountSwitch.accepts(previous, state.account, state.accountKind))
        assertFalse(state.authorized(old))
        observe("43")
        assertTrue(BinanceAccountSwitch.accepts(previous, state.account, state.accountKind))
        assertFalse(state.authorized(old))
        val next = state.grant(continuous = true)
        observe("42")
        assertFalse(state.authorized(next))
        assertFalse(state.authorized(old))
    }
}
