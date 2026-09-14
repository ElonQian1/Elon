package com.elon.app.grid.wallet

import com.elon.app.grid.host.BinanceHostConsent
import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceWalletStateTest {
    private var time=1_000L
    private val state=BinanceWalletState({time},{1_700_000_000_000+time})
    private val request="d".repeat(64)
    private fun bind()=state.bind("123","sub")
    @Test fun identityDeadlineMatchesFreshnessIncludingClockRollback() {
        assertEquals(0L,state.identityRemainingMs());bind();assertEquals(300_000L,state.identityRemainingMs())
        time+=299_999;assertTrue(state.identityFresh());assertEquals(1L,state.identityRemainingMs())
        time++;assertFalse(state.identityFresh());assertEquals(0L,state.identityRemainingMs())
        bind();time--;assertFalse(state.identityFresh());assertEquals(0L,state.identityRemainingMs())
    }
    private fun event(kind:String="wallet",account:String="123",rows:List<Any?> = listOf(mapOf("type" to "FUTURE","active" to true,"balance" to "1.23000000000000000001")))=mapOf(
        "schema" to BinanceWalletState.OBSERVATION,"request" to request,"token" to "doc_fixture_1","kind" to kind,
        "account" to account,"account_kind" to "sub","quote_asset" to "USDT","wallets" to rows)
    @Test fun gridTokenIsNotWalletPermissionAndWalletCanBeEmpty() {
        bind();assertFalse(state.authorized("a".repeat(64)))
        val grant=state.grant();state.start(request,"read",grant);state.accept(event(rows=emptyList()))
        val data=StrictJson.parse(state.reply(grant));assertEquals("fresh",data["status"]);assertEquals(emptyList<Any>(),data["wallets"])
    }
    @Test fun firstObservedIdentityDoesNotLoseTheOutstandingVerificationRequest() {
        state.start(request,"identify");bind()
        assertTrue(state.identifying)
        assertTrue(state.accept(mapOf("schema" to BinanceWalletState.OBSERVATION,"request" to request,"kind" to "identity","account" to "123","account_kind" to "sub")))
        assertFalse(state.identifying);assertTrue(state.identityFresh())
    }
    @Test fun revocationCannotBeUndoneByRenewalOrALateRead() {
        bind();val grant=state.grant();state.start(request,"read",grant);state.clear()
        assertThrows(IllegalArgumentException::class.java){state.renew(grant)}
        assertFalse(state.accept(event()));assertFalse(state.authorized(grant))
    }
    @Test fun decimalPrecisionMissingAndInactiveStayDistinct() {
        bind();val grant=state.grant();state.start(request,"read",grant)
        state.accept(event(rows=listOf(mapOf("type" to "FUTURE","active" to true,"balance" to "1.23000000000000000001"),mapOf("type" to "NEW_WALLET","active" to false,"balance" to null))))
        val raw=state.reply(grant);assertTrue(raw.contains("1.23000000000000000001"));assertTrue(raw.contains("null"));assertFalse(raw.contains("\"123\""))
    }
    @Test fun accountChangeAndRevocationRejectLateResponses() {
        bind();val grant=state.grant();state.start(request,"read",grant);state.bind("456","sub")
        assertFalse(state.authorized(grant));assertFalse(state.accept(event()));assertEquals(0,state.walletCount)
        val next=state.grant();state.start(request,"read",next);state.clear();assertFalse(state.accept(event(account="456")))
    }
    @Test fun responseMustMatchOriginatingScopeEvenWithoutObservedSwitch() {
        bind();val grant=state.grant();state.start(request,"read",grant)
        assertThrows(IllegalArgumentException::class.java){state.accept(event(account="456"))}
    }
    @Test fun duplicateWalletAndNumericAmountAreRejected() {
        bind();val grant=state.grant();state.start(request,"read",grant)
        assertThrows(IllegalArgumentException::class.java){state.accept(event(rows=listOf(mapOf("type" to "FUTURE","active" to true,"balance" to 1))))}
        state.fail(request,"response_invalid");state.start(request,"read",grant)
        val row=mapOf("type" to "FUTURE","active" to true,"balance" to "1")
        assertThrows(IllegalArgumentException::class.java){state.accept(event(rows=listOf(row,row)))}
    }
    @Test fun staleAndExpiryCannotBecomeFreshOrZero() {
        bind();val grant=state.grant();state.start(request,"read",grant);state.accept(event())
        time+=300_001;assertEquals("stale",StrictJson.parse(state.reply(grant))["status"])
        time+=600_000;assertFalse(state.authorized(grant))
    }
    @Test fun pendingFailureDoesNotReturnAnEmptySuccessfulWallet() {
        bind();val grant=state.grant();state.start(request,"read",grant);state.fail(request,"rate_limited")
        val data=StrictJson.parse(state.reply(grant));assertEquals("error",data["status"]);assertEquals("rate_limited",data["error"])
    }
    @Test fun separateConsentPurposeCannotAcceptTheExistingGridConsent() {
        val owner="a".repeat(64);val account=BinanceHostState.digest("123")
        val grid=BinanceHostConsent.encode(owner,account,"sub")
        assertFalse(BinanceHostConsent.matches(grid,owner,account,"sub",BinanceWalletState.PURPOSE))
        val wallet=BinanceHostConsent.encode(owner,account,"sub",BinanceWalletState.PURPOSE)
        assertFalse(BinanceHostConsent.matches(wallet,owner,account,"sub"))
        assertTrue(BinanceHostConsent.matches(wallet,owner,account,"sub",BinanceWalletState.PURPOSE))
    }
}
