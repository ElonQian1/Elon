package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatReadAloudChunkPolicyTest {
    @Test
    fun keepsTheWholeAnswerAcrossBoundedSpeechChunks() {
        val source = "第一句话。第二句话比较长，需要继续朗读。第三句话。"

        val chunks = WebChatReadAloudChunkPolicy.chunks(source, maxChars = 12)

        assertTrue(chunks.isNotEmpty())
        assertTrue(chunks.all { it.length <= 12 })
        assertEquals(source.replace(" ", ""), chunks.joinToString("").replace(" ", ""))
    }

    @Test
    fun rejectsBlankText() {
        assertTrue(WebChatReadAloudChunkPolicy.chunks("   ").isEmpty())
    }
}
