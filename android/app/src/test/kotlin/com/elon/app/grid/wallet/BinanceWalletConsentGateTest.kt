package com.elon.app.grid.wallet

import org.junit.Assert.*
import org.junit.Test

class BinanceWalletConsentGateTest {
    private var now=1_000L
    private val gate=BinanceWalletConsentGate {now}
    private val account=BinanceWalletConsentGate.Identity("owner-a","account-a","sub")
    private val none=BinanceWalletConsentGate.Decision.None
    private val refresh=BinanceWalletConsentGate.Decision.Refresh
    private val allow=BinanceWalletConsentGate.Decision.Approve(account)
    private fun shown() {gate.setActive(true);assertEquals(none,gate.observe(account,true,false,false))}
    private fun expiredClick() {shown();assertEquals(refresh,gate.click(account,false,false));assertTrue(gate.waiting)}

    @Test fun openingAndRefreshingNeverGrantButFreshHumanClickDoes() {
        shown();repeat(3){assertEquals(none,gate.observe(account,true,false,false))}
        assertEquals(allow,gate.click(account,true,false))
    }
    @Test fun expiredClickCompletesOnceAfterSameIdentityRefresh() {
        expiredClick();assertEquals(none,gate.observe(account,false,true,false))
        assertEquals(allow,gate.observe(account,true,false,false));assertFalse(gate.waiting)
        assertEquals(none,gate.observe(account,true,false,false))
    }
    @Test fun duplicateClickCannotRepeatRequestOrExtendDeadline() {
        expiredClick();now+=10_000
        assertEquals(none,gate.click(account,false,true));assertEquals(21_000L,gate.remaining())
    }
    @Test fun everyIdentityDimensionChangeRequiresAnotherReview() {
        for(next in listOf(account.copy(owner="owner-b"),account.copy(account="account-b"),account.copy(kind="primary"))) {
            expiredClick();assertEquals(none,gate.observe(next,true,false,false));assertFalse(gate.waiting)
            assertTrue(gate.notice!!.contains("账户发生变化"))
        }
    }
    @Test fun clearedIdentityCannotRecoverPendingConsent() {
        expiredClick();assertEquals(none,gate.observe(null,false,false,false))
        assertEquals(none,gate.observe(account,true,false,false));assertFalse(gate.waiting)
    }
    @Test fun timeoutIncludingExactBoundaryAndClockRollbackRejectLateIdentity() {
        for(delta in listOf(31_000L,31_001L,-1L)) {
            expiredClick();now+=delta
            assertEquals(none,gate.observe(account,true,false,false));assertFalse(gate.waiting)
            assertTrue(gate.notice!!.contains("超时"))
        }
    }
    @Test fun responseJustBeforeDeadlineStillCompletes() {
        expiredClick();now+=30_999;assertEquals(allow,gate.observe(account,true,false,false))
    }
    @Test fun failureCannotBecomeLaterAutomaticApproval() {
        expiredClick();assertEquals(none,gate.observe(account,false,false,true))
        assertFalse(gate.waiting);assertEquals(none,gate.observe(account,true,false,false))
    }
    @Test fun backgroundOrFocusLossCancelsAndReturningNeverGrants() {
        expiredClick();gate.setActive(false);assertFalse(gate.waiting)
        assertEquals(none,gate.click(account,true,false));gate.setActive(true)
        assertEquals(none,gate.observe(account,true,false,false))
    }
    @Test fun noPreviouslyDisplayedIdentityMeansRefreshWithoutApprovalIntent() {
        gate.setActive(true);assertEquals(refresh,gate.click(account,false,false));assertFalse(gate.waiting)
        assertEquals(none,gate.observe(account,true,false,false));assertEquals(allow,gate.click(account,true,false))
    }
    @Test fun explicitRetryCancelsEarlierClick() {
        expiredClick();gate.cancel();assertEquals(none,gate.observe(account,true,false,false))
    }
}
