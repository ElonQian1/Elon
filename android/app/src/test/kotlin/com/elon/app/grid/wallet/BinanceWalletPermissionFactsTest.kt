package com.elon.app.grid.wallet

import org.junit.Assert.*
import org.junit.Test

class BinanceWalletPermissionFactsTest {
    @Test fun distinguishesRecordedConsentFromCurrentPermission() {
        assertEquals("not_granted", facts(false,false,false,false)["status"])
        assertEquals("identity_required", facts(true,false,false,false)["status"])
        assertEquals("account_mismatch", facts(true,true,false,false)["status"])
        assertEquals("resume_required", facts(true,true,true,false)["status"])
        assertEquals("ready", facts(true,true,true,true)["status"])
    }
    @Test fun contradictoryInputsNeverElevateAuthority() {
        for(mask in 0..15) {
            val recorded=mask and 1 != 0;val identity=mask and 2 != 0
            val matches=mask and 4 != 0;val active=mask and 8 != 0
            val result=facts(recorded,identity,matches,active)
            assertEquals(recorded&&identity&&matches,result["consent_matches_current_identity"])
            assertEquals(recorded&&identity&&matches&&active,result["current_grant_active"])
            assertEquals(recorded&&identity&&matches&&active,result["status"]=="ready")
        }
    }
    @Test fun outputContainsOnlyFixedSchemaStateAndBooleans() {
        val value=facts(true,true,true,true)
        assertEquals(setOf("schema","status","consent_recorded","identity_verified","consent_matches_current_identity","current_grant_active"),value.keys)
        assertEquals("yilong.binance_wallet_permission_facts.v1",value["schema"])
        assertTrue(value.filterKeys {it !in setOf("schema","status")}.values.all {it is Boolean})
    }
    private fun facts(recorded:Boolean,identity:Boolean,matches:Boolean,active:Boolean)=
        BinanceWalletPermissionFacts.describe(recorded,identity,matches,active)
}
