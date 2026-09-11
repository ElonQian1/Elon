package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class WebChatSourceLinkPolicyTest {
    private val file = WebChatConversationFile("m:0", "m", "document", "source", "assistant", "",
        sourceUrl = "https://docs.example.test/document/fixture?tab=one#section")
    private val index = WebChatConversationFileIndex("/c/fixture", "mcp_source", listOf(file), false, 1000)

    @Test fun validSourceKeepsItsQueryAndFragment() {
        assertEquals(file.sourceUrl, WebChatSourceLinkPolicy.normalize(" ${file.sourceUrl} "))
        assertEquals(file.sourceUrl, WebChatSourceLinkPolicy.currentUrl(file, index, 2000))
    }

    @Test fun executableLocalCredentialBearingAndMalformedLinksAreRejected() {
        for (url in listOf("javascript:alert(1)", "intent://x", "file:///sdcard/a", "data:text/plain,a",
                "http://docs.example.test/a", "https://user:password@docs.example.test/a",
                "https://docs.example.test:8080/a", "https://docs.example.test\\@evil.test/a",
                "https://docs.example.test/a\nb", "https://docs.example.test/%xy", "https://",
                "https://docs.example.test/" + "a".repeat(8192))) {
            assertNull(url.take(80), WebChatSourceLinkPolicy.normalize(url))
        }
    }

    @Test fun staleReplacedClearedAndWrongKindSelectionsCannotOpen() {
        assertNull(WebChatSourceLinkPolicy.currentUrl(file, null, 2000))
        assertNull(WebChatSourceLinkPolicy.currentUrl(file, index, 61000))
        assertNull(WebChatSourceLinkPolicy.currentUrl(file, index, 500))
        assertNull(WebChatSourceLinkPolicy.currentUrl(file, index.copy(files = emptyList()), 2000))
        assertNull(WebChatSourceLinkPolicy.currentUrl(file, index.copy(files = listOf(file.copy(sourceUrl =
            "https://docs.example.test/other"))), 2000))
        assertNull(WebChatSourceLinkPolicy.currentUrl(file.copy(kind = "file"), index, 2000))
        assertNull(WebChatSourceLinkPolicy.currentUrl(file.copy(downloadHandle = "download_" + "a".repeat(32)), index, 2000))
    }
}
