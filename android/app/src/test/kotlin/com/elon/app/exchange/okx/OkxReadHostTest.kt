package com.elon.app.exchange.okx

import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class OkxReadHostTest {
    private class Vault : OkxAccessVault {
        val values = mutableMapOf<String, OkxSavedAccess>()
        override fun read(owner: String) = values[owner]
        override fun save(owner: String, access: OkxSavedAccess) { values[owner] = access }
        override fun revoke(owner: String) { values.remove(owner) }
    }
    private fun owner(id: String, token: String = "test-token") = EskPlatformSession.fromPreferences(mapOf("auth_user_id" to id, "auth_token" to token), 100)!!
    private val credentials = OkxCredentials("test-key", "test-secret", "test-passphrase")
    private fun account(id: String = "100") = """{"code":"0","data":[{"uid":"$id","mainUid":"100","perm":"read_only"}]}"""
    @Test fun consentRequiredAndRevokeDeletesOnlyCurrentOwner() {
        val vault = Vault(); val user = owner("alice")
        val host = OkxReadHost(vault, { user }, OkxReadGateway { _, _ -> account() }) { 0 }
        assertThrows(OkxReadException::class.java) { host.resume() }
        val verified = host.verify(credentials)
        assertTrue(vault.values.isEmpty())
        val grant = host.approve(verified)
        assertEquals(grant, host.resume()); assertEquals(1, vault.values.size)
        host.revoke(grant)
        assertTrue(vault.values.isEmpty()); assertThrows(OkxReadException::class.java) { host.read(grant, null) }
    }
    @Test fun changedPlatformOwnerCannotApproveOldVerificationOrRestoreOldKey() {
        val vault = Vault(); var user = owner("alice")
        val host = OkxReadHost(vault, { user }, OkxReadGateway { _, _ -> account() }) { 0 }
        val proof = host.verify(credentials); user = owner("bob")
        assertThrows(OkxReadException::class.java) { host.approve(proof) }
        user = owner("alice"); host.approve(host.verify(credentials)); user = owner("bob")
        assertThrows(OkxReadException::class.java) { host.resume() }
    }
    @Test fun lateNetworkResultIsRejectedAfterLogoutAndRelogin() {
        var user = owner("alice"); var rotate = false
        val gateway = OkxReadGateway { _, request ->
            if (request is OkxReadRequest.Pending) { if (rotate) user = owner("alice", "rotated-token"); """{"code":"0","data":[]}""" } else account()
        }
        val host = OkxReadHost(Vault(), { user }, gateway) { 0 }
        val grant = host.approve(host.verify(credentials)); rotate = true
        assertEquals(OkxReadFailure.CONNECTION_CHANGED, assertThrows(OkxReadException::class.java) { host.read(grant, null) }.reason)
    }
    @Test fun actualExchangeAccountMismatchRejectsRead() {
        var uid = "100"
        val host = OkxReadHost(Vault(), { owner("alice") }, OkxReadGateway { _, _ -> account(uid) }) { 0 }
        val grant = host.approve(host.verify(credentials)); uid = "101"
        assertEquals(OkxReadFailure.ACCOUNT_CHANGED, assertThrows(OkxReadException::class.java) { host.read(grant, null) }.reason)
        assertThrows(OkxReadException::class.java) { host.read(grant, null) }
    }
    @Test fun validEmptyInventoryRequiresBothAccountChecks() {
        var accountCalls = 0
        val gateway = OkxReadGateway { _, request -> if (request is OkxReadRequest.Pending) """{"code":"0","data":[]}""" else { accountCalls++; account() } }
        val host = OkxReadHost(Vault(), { owner("alice") }, gateway) { 0 }
        val grant = host.approve(host.verify(credentials))
        val result = StrictJson.parse(host.read(grant, null))
        assertEquals(3, accountCalls); assertEquals(true, result["complete"]); assertEquals(emptyList<Any>(), result["bots"])
    }
    @Test fun historyUsesBothAccountChecksAndPreservesVersionOneReads() {
        var accountCalls = 0
        val paths = mutableListOf<String>()
        val host = OkxReadHost(Vault(), { owner("alice") }, OkxReadGateway { _, request ->
            paths.add(request.path)
            if (request is OkxReadRequest.Account) { accountCalls++; account() } else """{"code":"0","data":[]}"""
        }) { 0 }
        val grant = host.approve(host.verify(credentials))
        val page = StrictJson.parse(host.history(grant, "99"))
        assertEquals(3, accountCalls); assertEquals("99", page["after"]); assertEquals(true, page["complete"])
        assertEquals(OkxHistoryPage.SCHEMA, page["schema"])
        assertTrue(paths.any { it.contains("orders-algo-history") && it.endsWith("after=99") })
        assertEquals(OkxReadProtocol.SCHEMA, StrictJson.parse(host.read(grant, null))["schema"])
    }
    @Test fun historyCannotReturnAfterRevokeOrPlatformSessionRotation() {
        var user = owner("alice")
        var revoke: (() -> Unit)? = null
        val host = OkxReadHost(Vault(), { user }, OkxReadGateway { _, request ->
            if (request is OkxReadRequest.History) { revoke?.invoke(); """{"code":"0","data":[]}""" } else account()
        }) { 0 }
        var grant = host.approve(host.verify(credentials))
        revoke = { host.revoke(grant) }
        assertEquals(OkxReadFailure.CONNECTION_CHANGED, assertThrows(OkxReadException::class.java) { host.history(grant, "") }.reason)
        grant = host.approve(host.verify(credentials))
        revoke = { user = owner("alice", "changed") }
        assertEquals(OkxReadFailure.CONNECTION_CHANGED, assertThrows(OkxReadException::class.java) { host.history(grant, "") }.reason)
    }
    @Test fun historyRejectsExchangeAccountChangeAfterPageWasRead() {
        var uid = "100"
        val host = OkxReadHost(Vault(), { owner("alice") }, OkxReadGateway { _, request ->
            if (request is OkxReadRequest.History) { uid = "101"; """{"code":"0","data":[]}""" } else account(uid)
        }) { 0 }
        val grant = host.approve(host.verify(credentials))
        assertEquals(OkxReadFailure.ACCOUNT_CHANGED, assertThrows(OkxReadException::class.java) { host.history(grant, "") }.reason)
    }
}
