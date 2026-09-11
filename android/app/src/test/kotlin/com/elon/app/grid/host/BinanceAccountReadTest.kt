package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceAccountReadTest {
    private var clock=1000L
    private val epoch=1_788_800_000_000L
    private fun payload(account:String="42",kind:String="sub")=StrictJson.encode(mapOf(
        "kind" to "list","account" to account,"account_kind" to kind,"rows" to emptyList<Any>()))
    private fun state()=BinanceHostState({clock},{epoch}).apply{accept(payload())}
    @Test fun emptyAccountListStillCarriesSameOpaqueManagementIdentity(){
        val s=state();val grant=s.grant(true);val r=StrictJson.parse(s.replyVersion(grant,3))
        assertEquals("yilong.binance_host_read.v3",r["schema"]);assertEquals(s.account,r["account"])
        assertEquals(BinanceHostState.digest("42"),r["account"]);assertEquals("sub",r["account_kind"])
        assertEquals(emptyList<Any>(),r["rows"])
        assertFalse(s.reply(grant).contains("\"account\""));assertFalse(s.reply(grant,true).contains("\"account\""))
    }
    @Test fun accountAndKindChangesRejectOriginalGrant(){
        val s=state();val old=s.grant(true);s.accept(payload("43"))
        assertThrows(IllegalArgumentException::class.java){s.replyVersion(old,3)}
        val next=s.grant(true);assertEquals(BinanceHostState.digest("43"),StrictJson.parse(s.replyVersion(next,3))["account"])
        s.accept(payload("43","primary"));assertThrows(IllegalArgumentException::class.java){s.replyVersion(next,3)}
    }
    @Test fun expiryAndRevocationNeverLeakBoundIdentity(){
        val s=state();val old=s.grant(true);clock+=300_000
        assertEquals("stale",StrictJson.parse(s.replyVersion(old,3))["status"])
        s.revoke(old);assertThrows(IllegalArgumentException::class.java){s.replyVersion(old,3)}
        assertThrows(IllegalArgumentException::class.java){s.replyVersion(old,4)}
    }
}
