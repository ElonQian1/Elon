package com.elon.app.chatgptweb

import com.elon.app.WebChatTextBlock
import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatTextBlockContinuityTest {
    private val source = WebChatTextBlock("writing-a", "writing", "Draft", "", "body\n", true, "a1")
    private val oldPart = ChatGptWebMessagePart("writing_block", "Draft", textBlock = source)
    private val domPart = ChatGptWebMessagePart("code", "Code", textBlock =
        WebChatTextBlock("code-0", "code", "", "", "body\n", true))
    private fun message(parts: List<ChatGptWebMessagePart>) = ChatGptWebMessage(
        id = "a1", role = "assistant", content = "outside block", state = "completed", parts = parts,
    )

    @Test fun exactUniqueBodyRetainsWritingIdentityAndDoesNotRemoveOtherParts() {
        val image = ChatGptWebMessagePart("image", "Image")
        val result = WebChatTextBlockContinuity.merge(message(listOf(oldPart)), message(listOf(image, domPart)))
        assertEquals(listOf(image, oldPart), result.parts)
    }

    @Test fun changedAbsentDuplicateAndIncompleteBodiesNeverInheritOldBlocks() {
        val previous = message(listOf(oldPart))
        for (parts in listOf(emptyList(), listOf(domPart.copy(textBlock = domPart.textBlock!!.copy(content = "new"))),
            listOf(domPart, domPart), listOf(domPart.copy(textBlock = domPart.textBlock!!.copy(complete = false))))) {
            val incoming = message(parts)
            assertEquals(incoming, WebChatTextBlockContinuity.merge(previous, incoming))
        }
        val incoming = message(listOf(domPart))
        assertEquals(incoming, WebChatTextBlockContinuity.merge(message(listOf(oldPart, oldPart)), incoming))
    }

    @Test fun identityRoleAndStreamingChangesNeverCarryBlockMetadata() {
        val previous = message(listOf(oldPart))
        for (incoming in listOf(message(listOf(domPart)).copy(id = "a2"),
            message(listOf(domPart)).copy(role = "user"), message(listOf(domPart)).copy(state = "streaming"))) {
            assertEquals(incoming, WebChatTextBlockContinuity.merge(previous, incoming))
        }
        val incoming = message(listOf(domPart))
        assertEquals(incoming, WebChatTextBlockContinuity.merge(previous.copy(state = "streaming"), incoming))
    }

    @Test fun NewProviderMetadataAndExplicitLanguageTakePrecedence() {
        for (part in listOf(oldPart.copy(textBlock = source.copy(title = "New title")),
            domPart.copy(textBlock = domPart.textBlock!!.copy(language = "python")))) {
            val incoming = message(listOf(part))
            assertEquals(incoming, WebChatTextBlockContinuity.merge(message(listOf(oldPart)), incoming))
        }
    }
}
