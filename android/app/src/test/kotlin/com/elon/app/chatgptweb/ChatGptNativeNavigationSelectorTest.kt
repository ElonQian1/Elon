package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptNativeNavigationSelectorTest {
    @Test
    fun selectorsKeepProviderIdsAndReadableLabelsStable() {
        val conversation = ChatGptWebConversation("demo/unsafe", " 复杂\n内容 ", "/c/demo", false)
        val feature = ChatGptWebFeature("feature_library", "文件库", "library", false)
        val option = ChatGptWebComposerOption("model_fast", "快速", false, "menuitemradio")

        assertEquals(
            "chatgpt-conversation:demo_unsafe:复杂 内容",
            ChatGptNativeNavigationSelector.conversation(conversation),
        )
        assertEquals(
            "chatgpt-feature:feature_library:文件库",
            ChatGptNativeNavigationSelector.feature(feature),
        )
        assertEquals(
            "chatgpt-composer-option:model:model_fast:快速",
            ChatGptNativeNavigationSelector.composerOption("model", option),
        )
        assertEquals(
            "chatgpt-composer-options:attachments",
            ChatGptNativeNavigationSelector.composerDialog("attachments"),
        )
        assertTrue(ChatGptNativeNavigationSelector.SCHEMA.endsWith(".v1"))
    }
}
