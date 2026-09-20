package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class AiConversationContinuationPromptTest {
    @Test fun acceptsOnlyOrdinaryFreshPersonalRoute() {
        assertTrue(AiConversationContinuationPrompt.freshRoute("https://chatgpt.com/"))
        listOf("https://chatgpt.com/c/old", "https://chatgpt.com/g/g-p-test/project",
            "https://chatgpt.com/?temporary-chat=true", "https://evil.test/",
            "https://user@chatgpt.com/", "http://chatgpt.com/", "https://chatgpt.com/#old")
            .forEach { assertFalse(it, AiConversationContinuationPrompt.freshRoute(it)) }
    }

    @Test fun preservesSelectedMarkdownAndNeverAddsMissingContext() {
        val rows = listOf(ChatMessage("user", "chosen question"), ChatMessage("assistant", "**answer**\n```kotlin\nval a = 1\n```"))
        val prompt = AiConversationContinuationPrompt.build(rows)
        assertTrue(prompt.contains("chosen question"))
        assertTrue(prompt.contains("**answer**\n```kotlin\nval a = 1\n```"))
        assertTrue(prompt.endsWith("我的问题："))
        assertFalse(prompt.contains("unselected message"))
    }

    @Test(expected = IllegalArgumentException::class) fun refusesOversizeInsteadOfTruncating() {
        AiConversationContinuationPrompt.build(listOf(ChatMessage("user", "x".repeat(21_000))))
    }
}
