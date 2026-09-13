package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSnapshotPrivacyPolicyTest {
    @Test
    fun ordinarySnapshotsKeepTheirExistingCache() {
        listOf(
            "https://chatgpt.com/",
            "https://chatgpt.com:443/c/synthetic",
            "https://chatgpt.com/g/g-p-synthetic/c/synthetic?model=auto",
            "https://chatgpt.com/?temporary-chat=false",
            "https://chatgpt.com/?temporary-chat=false&model=auto",
        ).forEach { assertTrue(it, WebChatSnapshotPrivacyPolicy.canPersist("chatgpt", it)) }
    }

    @Test
    fun temporarySnapshotsCannotBeSavedOrRestored() {
        listOf(
            "https://chatgpt.com/?temporary-chat=true",
            "https://chatgpt.com/?model=auto&temporary-chat=true#composer",
            "https://chatgpt.com/c/synthetic?temporary-chat=true",
            "https://chatgpt.com/?%74emporary-chat=%74rue",
            "https://chatgpt.com/?temporary-chat=false&temporary-chat=true",
            "https://chatgpt.com/?temporary-chat=true&temporary-chat=false",
        ).forEach { assertFalse(it, WebChatSnapshotPrivacyPolicy.canPersist("chatgpt", it)) }
    }

    @Test
    fun ambiguousOrUntrustedPrivacyStateIsNotPersistent() {
        listOf(
            "", "about:blank", "/?temporary-chat=true",
            "http://chatgpt.com/", "https://chatgpt.com.example/",
            "https://user@chatgpt.com/", "https://chatgpt.com:444/",
            "https://chatgpt.com/?temporary-chat", "https://chatgpt.com/?temporary-chat=",
            "https://chatgpt.com/?temporary-chat=unknown",
            "https://chatgpt.com/?temporary-chat=false&temporary-chat=false",
            "https://chatgpt.com/?temporary-chat=%ZZ",
        ).forEach { assertFalse(it, WebChatSnapshotPrivacyPolicy.canPersist("chatgpt", it)) }
    }

    @Test
    fun googleCachePolicyIsUnchanged() {
        assertTrue(WebChatSnapshotPrivacyPolicy.canPersist("google", "https://www.google.com/search?q=synthetic"))
        assertTrue(WebChatSnapshotPrivacyPolicy.canPersist("google", "about:blank"))
        assertTrue(WebChatSnapshotPrivacyPolicy.canPersist("google", "https://www.google.com/?temporary-chat=true"))
    }
}
