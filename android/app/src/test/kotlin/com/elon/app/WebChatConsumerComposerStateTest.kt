package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConsumerComposerStateTest {
    @Test
    fun chatGptShowsActualControlsOnlyAfterTheComposerIsReady() {
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)

        val loading = WebChatConsumerComposerStateResolver.resolve(
            provider,
            state = "loading",
            composerReady = false,
            attachmentSupported = true,
        )
        val ready = WebChatConsumerComposerStateResolver.resolve(
            provider,
            state = "ready",
            composerReady = true,
            attachmentSupported = true,
        )

        assertFalse(loading.attachmentVisible)
        assertFalse(loading.toolsVisible)
        assertFalse(loading.submissionEnabled)
        assertTrue(loading.inputHint.startsWith("正在连接"))
        assertTrue(ready.attachmentVisible)
        assertTrue(ready.toolsVisible)
        assertTrue(ready.submissionEnabled)
        assertEquals("输入内容", ready.inputHint)
    }

    @Test
    fun googleNeverPretendsToSupportChatGptComposerControls() {
        val state = WebChatConsumerComposerStateResolver.resolve(
            WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB),
            state = "ready",
            composerReady = true,
            attachmentSupported = false,
        )

        assertFalse(state.attachmentVisible)
        assertFalse(state.toolsVisible)
        assertTrue(state.submissionEnabled)
    }

    @Test
    fun explicitLoginRequirementIsNotPresentedAsAConnectionDelay() {
        val state = WebChatConsumerComposerStateResolver.resolve(
            WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            state = "login_required",
            composerReady = false,
            attachmentSupported = false,
        )

        assertEquals("当前网页要求登录，输入内容将保留", state.inputHint)
        assertFalse(state.submissionEnabled)
    }

    @Test
    fun staleComposerFlagDoesNotEnableSubmissionAfterAnError() {
        val state = WebChatConsumerComposerStateResolver.resolve(
            WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            state = "error",
            composerReady = true,
            attachmentSupported = true,
        )

        assertFalse(state.submissionEnabled)
        assertFalse(state.attachmentVisible)
        assertFalse(state.toolsVisible)
    }
}
