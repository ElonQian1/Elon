package com.elon.app.exchange.okx

import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class OkxRecordsHostTest {
    private class Vault:OkxAccessVault {
        var saved:OkxSavedAccess?=null
        override fun read(owner:String)=saved
        override fun save(owner:String,access:OkxSavedAccess){saved=access}
        override fun revoke(owner:String){saved=null}
    }
    private fun owner(token:String="first")=EskPlatformSession.fromPreferences(mapOf("auth_user_id" to "alice","auth_token" to token),100)!!
    private fun account(id:String="100")="""{"code":"0","data":[{"uid":"$id","mainUid":"100","perm":"read_only"}]}"""
    @Test fun reportChecksAccountBeforeAndAfterAndKeepsOldReads() {
        var accounts=0
        val gateway=OkxReadGateway{_,request->when(request){
            is OkxReadRequest.Account->{accounts++;account()}
            is OkxReadRequest.Detail->OkxRecordFixtures.response(listOf(mapOf("algoId" to "123","algoOrdType" to "contract_grid","instId" to OkxRecordFixtures.SYMBOL)))
            is OkxReadRequest.Records->OkxRecordFixtures.response(listOf(OkxRecordFixtures.order()))
            else->OkxRecordFixtures.response(emptyList())
        }}
        val host=OkxReadHost(Vault(),{owner()},gateway){0};val grant=host.approve(host.verify(OkxRecordFixtures.credentials))
        val result=StrictJson.parse(host.records(grant,"orders","123",OkxRecordFixtures.SYMBOL,""))
        assertEquals(3,accounts);assertEquals(OkxRecordsPage.SCHEMA,result["schema"]);assertEquals("orders",result["kind"])
        assertEquals("90",result["next_after"]);assertEquals("123",result["strategy_id"])
        assertEquals(OkxReadProtocol.SCHEMA,StrictJson.parse(host.read(grant,null))["schema"])
        assertEquals(OkxHistoryPage.SCHEMA,StrictJson.parse(host.history(grant,""))["schema"])
    }
    @Test fun lateRecordsAreRejectedAfterRevocationSessionRotationOrExchangeSwitch() {
        for(mode in listOf("revoke","session","account")) {
            var user=owner();var uid="100";var onRead:(()->Unit)?=null
            val host=OkxReadHost(Vault(),{user},OkxReadGateway{_,request->when(request){
                is OkxReadRequest.Account->account(uid)
                is OkxReadRequest.Detail->OkxRecordFixtures.response(listOf(mapOf("algoId" to "123","algoOrdType" to "contract_grid","instId" to OkxRecordFixtures.SYMBOL)))
                else->{onRead?.invoke();OkxRecordFixtures.response(emptyList())}
            }}){0}
            val grant=host.approve(host.verify(OkxRecordFixtures.credentials))
            onRead={when(mode){"revoke"->host.revoke(grant);"session"->user=owner("second");else->uid="101"}}
            val error=assertThrows(OkxReadException::class.java){host.records(grant,"orders","123",OkxRecordFixtures.SYMBOL,"")}
            assertEquals(if(mode=="account")OkxReadFailure.ACCOUNT_CHANGED else OkxReadFailure.CONNECTION_CHANGED,error.reason)
        }
    }
}
