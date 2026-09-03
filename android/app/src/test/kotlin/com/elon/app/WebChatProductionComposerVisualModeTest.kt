package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProductionComposerVisualModeTest {
    @Test
    fun streamingAlwaysUsesTheStopAction() {
        assertEquals(
            WebChatProductionComposerVisualMode.STOP,
            resolve(streaming = true, hasText = true, hasAttachments = true, voiceMode = true),
        )
    }

    @Test
    fun expandedTextAndAttachmentsUseTheSendAction() {
        assertEquals(
            WebChatProductionComposerVisualMode.SEND,
            resolve(hasText = true),
        )
        assertEquals(
            WebChatProductionComposerVisualMode.SEND,
            resolve(hasAttachments = true, composerExpanded = false),
        )
    }

    @Test
    fun collapsedOrVoiceInputKeepsTheInputModeAction() {
        assertEquals(
            WebChatProductionComposerVisualMode.INPUT_MODE,
            resolve(hasText = true, composerExpanded = false),
        )
        assertEquals(
            WebChatProductionComposerVisualMode.INPUT_MODE,
            resolve(hasText = true, voiceMode = true),
        )
    }

    @Test
    fun activeDictationKeepsCompletionControlsAfterTranscriptAddsText() {
        assertEquals(
            WebChatProductionComposerVisualMode.INPUT_MODE,
            resolve(hasText = true, dictationActive = true),
        )
    }

    private fun resolve(
        streaming: Boolean = false,
        hasText: Boolean = false,
        hasAttachments: Boolean = false,
        voiceMode: Boolean = false,
        composerExpanded: Boolean = true,
        dictationActive: Boolean = false,
    ) = WebChatProductionComposerVisualModeResolver.resolve(
        streaming = streaming,
        hasText = hasText,
        hasAttachments = hasAttachments,
        voiceMode = voiceMode,
        composerExpanded = composerExpanded,
        dictationActive = dictationActive,
    )
}
