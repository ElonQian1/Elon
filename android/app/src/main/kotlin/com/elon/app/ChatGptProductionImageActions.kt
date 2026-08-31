package com.elon.app

internal class ChatGptProductionImageActions(
    private val composerTools: WebChatProductionComposerToolsCoordinator,
    private val activeController: () -> WebChatSocialController,
) {
    fun requestCreateImage() {
        composerTools.selectQuickAction(
            WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            WebChatProductionQuickComposerAction.IMAGE_GENERATION,
        )
    }

    fun openNativeFeature(feature: WebChatProductionFeature): Boolean =
        feature.kind == "images" && activeController().showNativeImageGallery()
}
