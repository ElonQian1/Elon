package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProviderPickerSheetTest {
    @Test
    fun currentProviderShowsModelAndAnonymousReadyState() {
        val options = webChatProviderPickerOptions(
            providers = WebChatProviderRegistry.available(),
            selectedProvider = WebChatProviderId.CHATGPT_WEB,
            currentModel = "GPT-5 Fast",
            currentState = "ready",
            authenticated = false,
            composerReady = true,
        )

        assertEquals("GPT-5 Fast · 访客会话", options.first().subtitle)
        assertEquals("点击切换", options.last().subtitle)
        assertTrue(options.first().selected)
        assertFalse(options.last().selected)
    }

    @Test
    fun selectedProviderReportsConnectionAndAccountStatesWithoutIdentityData() {
        assertEquals("正在连接", webChatProviderSessionLabel("loading", true, false))
        assertEquals("账号会话", webChatProviderSessionLabel("ready", true, true))
        assertEquals("连接异常", webChatProviderSessionLabel("error", false, false))
        assertEquals("需要登录", webChatProviderSessionLabel("login_required", false, false))
    }
}
