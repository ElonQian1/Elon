package com.elon.app

internal fun createChatGptWebAcceptanceAttachmentActions(
    feature: MainSocialAiChatFeature,
    attachments: MainPendingAttachmentActions,
): ChatGptWebAcceptanceAttachmentNativeActions = ChatGptWebAcceptanceAttachmentNativeActions(
    isChatModeActive = feature::isChatModeActive,
    webChatState = feature::webChatState,
    stageFixture = attachments::stageChatGptWebAcceptanceFixture,
    removeFixture = {
        val nativeRemoved = attachments.removeChatGptWebAcceptanceFixture()
        val webRemoved = feature.discardWebChatAcceptanceAttachmentSend()
        nativeRemoved || webRemoved
    },
    pendingCount = attachments::pendingAttachmentCount,
    fixtureStaged = attachments::hasChatGptWebAcceptanceFixture,
    attachmentSendPhase = feature::webChatAttachmentPhase,
)
