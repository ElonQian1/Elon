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

    @Test
    fun composerRequestClearsOnlyTheRequestedStaleSection() {
        val state = ChatGptWebObservedState()
        state.accept(composerEvent("model", "快速"))
        state.accept(composerEvent("tools", "网页搜索"))

        state.beginComposerRequest("model")

        assertTrue("model" !in state.snapshot().composerSections)
        assertEquals("网页搜索", state.snapshot().composerSections.getValue("tools").single().label)
    }

    private fun composerEvent(section: String, label: String) = ChatGptWebEvent.ComposerControls(
        section = section,
        currentModel = "5.6 Sol 轻度",
        options = listOf(ChatGptWebComposerOption("${section}_option", label, false, "menuitem")),
    )
}
