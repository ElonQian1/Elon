package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebUploadPolicyTest {
    @Test
    fun preservesAUsableExtensionWhenTheDisplayNameDoesNotHaveOne() {
        assertEquals(
            "需求说明.pdf",
            ChatGptWebUploadPolicy.stagedName("需求说明", "attachment_1.pdf", 0),
        )
    }

    @Test
    fun sanitizesPathCharactersAndBoundsTheName() {
        val name = ChatGptWebUploadPolicy.stagedName("../危险/文件?.txt", "fallback.txt", 0)

        assertTrue(name.length <= 120)
        assertTrue('/' !in name && '\\' !in name && '?' !in name)
        assertTrue(name.endsWith(".txt"))
    }
}
