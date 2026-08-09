package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebObservedStateTest {
    @Test
    fun retainsNavigationComposerAndCommandObservations() {
        val state = ChatGptWebObservedState()
        state.accept(ChatGptWebEvent.ConversationList(listOf(
            ChatGptWebConversation("demo", "桥接验证", "/c/demo", active = true),
        )))
        state.accept(ChatGptWebEvent.FeatureNavigation(listOf(
            ChatGptWebFeature("feature_library", "文件库", "library", selected = false),
        )))
        state.accept(ChatGptWebEvent.ComposerControls(
            section = "model",
            currentModel = "5.6 Sol 轻度",
            options = listOf(ChatGptWebComposerOption("model_fast", "快速", false, "model")),
        ))
        state.accept(ChatGptWebEvent.CommandResult("list_conversations", true, ""))

        val snapshot = state.snapshot()
        assertEquals("/c/demo", snapshot.conversations.single().path)
        assertEquals("library", snapshot.features.single().kind)
        assertEquals("快速", snapshot.composerSections.getValue("model").single().label)
        assertTrue(snapshot.lastCommand?.ok == true)
        assertTrue(snapshot.updatedAtMs > 0)
    }
}
