package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class AccountIdentityPresentationTest {
    @Test
    fun masksPhoneAccountBeforeShowingBindingTarget() {
        assertEquals("156****92", maskedYilongAccount("15692409892"))
    }

    @Test
    fun masksGoogleEmailInProfileSummary() {
        val identities = listOf(
            LinkedLoginIdentity("identity-1", "google", "example@gmail.com", "Example"),
        )
        assertEquals("Google 已绑定 · ex***e@gmail.com", googleBindingSummary(identities))
        assertEquals("Google 未绑定 · 点击设置", googleBindingSummary(emptyList()))
        assertEquals("Google 暂未配置", googleBindingSummary(emptyList(), googleConfigured = false))
    }

    @Test
    fun mapsStableFederatedErrorCodesToActionableMessages() {
        assertEquals(
            "Google 登录尚未配置，暂时无法绑定",
            federatedAuthErrorMessage("google_oidc_not_configured", "raw"),
        )
        assertEquals(
            "这个 Google 账号已绑定到另一一龙账号，不能自动合并；请先在原账号解绑",
            federatedAuthErrorMessage("identity_owned_by_another_account", "raw"),
        )
    }
}
