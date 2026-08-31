package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConsumerStatusBannerTest {
    private val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
    private val google = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)

    @Test
    fun anonymousReadyChatDoesNotShowARecoveryBanner() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "ready")

        assertFalse(state.visible)
        assertFalse(state.retryVisible)
        assertFalse(state.officialVisible)
    }

    @Test
    fun connectionFailureOffersRetryWithoutDiscardingTheNativeComposer() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "error")

        assertTrue(state.visible)
        assertTrue(state.retryVisible)
        assertTrue(state.officialVisible)
        assertTrue(state.message.contains(chatGpt.displayName))
    }

    @Test
    fun reconnectingKeepsCachedConversationVisibleWithoutOfferingPrematureActions() {
        val state = WebChatConsumerRecoveryPolicy.resolve(
            provider = chatGpt,
            state = "loading",
            hasConversationContent = true,
        )

        assertTrue(state.visible)
        assertTrue(state.message.contains("当前对话已保留"))
        assertFalse(state.retryVisible)
        assertFalse(state.officialVisible)
    }

    @Test
    fun firstConnectionDoesNotDuplicateTheEmptyConversationStatus() {
        assertFalse(WebChatConsumerRecoveryPolicy.resolve(
            provider = chatGpt,
            state = "loading",
            hasConversationContent = false,
        ).visible)
    }

    @Test
    fun warmSessionRecoveryDoesNotCoverTheComposerWithAReconnectBanner() {
        val state = WebChatConsumerRecoveryPolicy.resolve(
            provider = chatGpt,
            state = "loading",
            hasConversationContent = true,
            warmSessionAvailable = true,
        )

        assertFalse(state.visible)
    }

    @Test
    fun commonNetworkFailuresUseConsumerFriendlyRecoveryText() {
        val offline = WebChatConsumerRecoveryPolicy.resolve(
            provider = google,
            state = "error",
            detail = "net::ERR_INTERNET_DISCONNECTED",
        )
        val timeout = WebChatConsumerRecoveryPolicy.resolve(
            provider = chatGpt,
            state = "error",
            detail = "net::ERR_TIMED_OUT",
        )

        assertEquals("网络不可用，请检查加速网络后重试", offline.message)
        assertEquals("${chatGpt.displayName}连接超时，请重试", timeout.message)
    }

    @Test
    fun explicitLoginRequirementOffersGuestRetryAndOptionalOfficialLogin() {
        val state = WebChatConsumerRecoveryPolicy.resolve(chatGpt, "login_required")

        assertTrue(state.visible)
        assertTrue(state.retryVisible)
        assertTrue(state.officialVisible)
        assertEquals("可尝试免费访客聊天，或登录账号", state.message)
        assertEquals("访客", state.retryLabel)
        assertEquals("登录", state.officialLabel)
    }

    @Test
    fun attachmentProgressTakesPriorityOverTransientToolFeedback() {
        val feedback = WebChatConsumerComposerOperationPolicy.commandAccepted(
            chatGpt,
            "chatgpt_start_dictation",
        )

        val state = WebChatConsumerComposerOperationPolicy.resolve(
            provider = chatGpt,
            attachmentPhase = "uploading",
            feedback = feedback,
        )

        assertTrue(state.visible)
        assertEquals("附件上传中，完成后会自动发送", state.message)
        assertFalse(state.retryVisible)
        assertFalse(state.officialVisible)
    }

    @Test
    fun attachmentFailureExplainsThatTheSelectionCanBeRetried() {
        val state = WebChatConsumerComposerOperationPolicy.resolve(
            provider = chatGpt,
            attachmentPhase = "failed",
            feedback = null,
        )

        assertEquals("附件发送失败，附件已保留，可重新发送", state.message)
    }

    @Test
    fun activeDictationKeepsAVisibleCompletionAndCancellationHint() {
        val state = WebChatConsumerComposerOperationPolicy.resolve(
            provider = chatGpt,
            attachmentPhase = "idle",
            feedback = null,
            dictationActive = true,
        )

        assertTrue(state.visible)
        assertEquals("正在听写，点蓝色勾完成，点×取消", state.message)
    }

    @Test
    fun commandFeedbackIsScopedToTheProviderThatAcceptedIt() {
        val feedback = WebChatConsumerComposerOperationPolicy.commandAccepted(
            chatGpt,
            "chatgpt_start_realtime_voice",
        )

        assertTrue(WebChatConsumerComposerOperationPolicy.resolve(chatGpt, "idle", feedback).visible)
        assertFalse(WebChatConsumerComposerOperationPolicy.resolve(google, "idle", feedback).visible)
    }

    @Test
    fun imageGenerationStatusSeparatesGenerationPreparationAndPreviewFailure() {
        val generating = WebChatConsumerComposerOperationPolicy.resolve(
            provider = chatGpt,
            attachmentPhase = "idle",
            feedback = null,
            imageGenerationActive = true,
            streaming = true,
        )
        val preparing = WebChatConsumerComposerOperationPolicy.resolve(
            provider = chatGpt,
            attachmentPhase = "idle",
            feedback = null,
            imageGenerationActive = true,
            imagePreviewState = "preparing",
        )
        val failed = WebChatConsumerComposerOperationPolicy.resolve(
            provider = chatGpt,
            attachmentPhase = "idle",
            feedback = null,
            imageGenerationActive = true,
            imagePreviewState = "failed",
        )

        assertEquals("正在创建图片…", generating.message)
        assertEquals("图片已生成，正在准备预览…", preparing.message)
        assertEquals("图片已生成，预览同步失败，可在“图像”中重试", failed.message)
        assertFalse(
            WebChatConsumerComposerOperationPolicy.resolve(
                provider = chatGpt,
                attachmentPhase = "idle",
                feedback = null,
                imageGenerationActive = false,
                imagePreviewState = "failed",
            ).visible,
        )
    }
}
