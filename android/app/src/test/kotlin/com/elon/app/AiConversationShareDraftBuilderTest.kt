package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class AiConversationShareDraftBuilderTest {
    private fun row(id: String, body: String) = ChatMessage("friend", body, id = "chatgpt_web:$id")

    @Test fun selectionKeepsSourceOrderAndDoesNotUseHiddenContent() {
        val first = row("a", "**Selected** first")
        val hidden = row("b", "Never share this private context")
        val last = row("c", "Selected last")
        val draft = AiConversationShareDraftBuilder.build(listOf(first, hidden, last), listOf(last, first), false)
        assertEquals(listOf(first.id, last.id), draft.messages.map { it.id })
        assertEquals(setOf(1), draft.gaps)
        assertFalse(draft.title.contains("private"))
        assertFalse(draft.summary.contains("private"))
        first.content = "Changed after preview"
        assertEquals("**Selected** first", draft.messages.first().content)
    }

    @Test fun rejectsChangedSelectionAndStreaming() {
        val selected = row("a", "before")
        val current = selected.copy(content = "after", revision = 2)
        assertThrows(IllegalArgumentException::class.java) {
            AiConversationShareDraftBuilder.build(listOf(current), listOf(selected), false)
        }
        assertThrows(IllegalArgumentException::class.java) {
            AiConversationShareDraftBuilder.build(listOf(selected), listOf(selected), true)
        }
    }

    @Test fun rejectsStatusesDuplicatesAndPartialWritingBlocks() {
        val status = row("status", "connecting")
        assertThrows(IllegalArgumentException::class.java) {
            AiConversationShareDraftBuilder.build(listOf(status), listOf(status), false)
        }
        val row = row("a", "text")
        assertThrows(IllegalArgumentException::class.java) {
            AiConversationShareDraftBuilder.build(listOf(row), listOf(row, row), false)
        }
        val partial = row.copy(webChatMessage = WebChatProductionMessage("chatgpt_web", "a", emptySet(), true,
            listOf(WebChatProductionContentPart("artifact", "document", textBlock =
                WebChatTextBlock("block", "writing", "document", "", "unfinished", false)))))
        assertThrows(IllegalArgumentException::class.java) {
            AiConversationShareDraftBuilder.build(listOf(partial), listOf(partial), false)
        }
    }

    @Test fun preservesMarkdownAndCompletedStructuredBlocks() {
        val content = "|Name|Value|\n|---|---|\n|A|1|\n```kotlin\nval x = 1\n```"
        val block = WebChatTextBlock("b", "code", "Example", "kotlin", "val x = 1", true)
        val source = row("a", content).copy(webChatMessage = WebChatProductionMessage("chatgpt_web", "a", emptySet(), true,
            listOf(WebChatProductionContentPart("code", "Example", textBlock = block))))
        val result = AiConversationShareDraftBuilder.build(listOf(source), listOf(source), false)
        assertEquals(content, result.messages.single().content)
        assertEquals(block, result.messages.single().webChatMessage!!.contentParts.single().textBlock)
    }
}
