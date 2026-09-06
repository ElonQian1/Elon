package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebComposerOptionSemanticsTest {
    @Test
    fun modelVersionAndServiceTierKeepDistinctProtocolSemantics() {
        assertTrue("model_version" in ChatGptWebComposerOptionSemantics.KNOWN)
        assertTrue("service_tier" in ChatGptWebComposerOptionSemantics.KNOWN)
        assertFalse(ChatGptWebComposerOptionSemantics.isAttachment("model_version"))
        assertFalse(ChatGptWebComposerOptionSemantics.isAttachment("service_tier"))
    }

    @Test
    fun attachmentSemanticsAreStableAndIndependentFromDisplayedLabels() {
        listOf(
            ChatGptWebComposerOptionSemantics.ATTACHMENT_CAMERA,
            ChatGptWebComposerOptionSemantics.ATTACHMENT_PHOTOS,
            ChatGptWebComposerOptionSemantics.ATTACHMENT_FILE,
        ).forEach { assertTrue(ChatGptWebComposerOptionSemantics.isAttachment(it)) }

        assertFalse(ChatGptWebComposerOptionSemantics.isAttachment(
            ChatGptWebComposerOptionSemantics.WEB_SEARCH,
        ))
        assertEquals(
            ChatGptWebComposerOptionSemantics.MODEL,
            ChatGptWebComposerOptionSemantics.fallback("model"),
        )
        assertEquals(
            ChatGptWebComposerOptionSemantics.TOOL,
            ChatGptWebComposerOptionSemantics.fallback("tools"),
        )
    }
}
