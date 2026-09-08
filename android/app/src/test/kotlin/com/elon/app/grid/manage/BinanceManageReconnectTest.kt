package com.elon.app.grid.manage

import org.junit.Assert.*
import org.junit.Test

class BinanceManageReconnectTest {
    @Test fun expiredDestroyedPageIsRecreatedBeforeAttaching() {
        var page:Any?=null;val next=Any();val events=mutableListOf<String>()
        assertTrue(reconnectBinanceManagePage(false,{events+="cancel"},{page},{events+="begin";page=next;true},{assertSame(next,it);events+="attach"},{events+="reload"}))
        assertEquals(listOf("cancel","begin","attach"),events)
    }
    @Test fun existingPageIsRefreshedOnlyAfterSuccessfulBegin() {
        val page=Any();val events=mutableListOf<String>()
        assertTrue(reconnectBinanceManagePage(false,{events+="cancel"},{page},{events+="begin";true},{events+="attach"},{assertSame(page,it);events+="reload"}))
        assertEquals(listOf("cancel","begin","attach","reload"),events)
    }
    @Test fun failedAccountRestoreDoesNotAttachOrNavigate() {
        val events=mutableListOf<String>()
        assertFalse(reconnectBinanceManagePage(false,{events+="cancel"},{Any()},{false},{events+="attach"},{events+="reload"}))
        assertEquals(listOf("cancel"),events)
    }
    @Test fun submittingCannotCancelOrBegin() {
        assertFalse(reconnectBinanceManagePage<Any>(true,{fail("cancel")},{fail("page");null},{fail("begin");true},{fail("attach")},{fail("reload")}))
    }
    @Test fun beginWithoutPageNeverClaimsStarted() {
        assertFalse(reconnectBinanceManagePage<Any>(false,{},{null},{true},{fail("attach")},{fail("reload")}))
    }
}
