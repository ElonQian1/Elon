package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebSessionRestorerTest {
    private val snapshot = ChatGptWebSnapshot(
        title = "Fixture", url = "https://chatgpt.com/c/fixture", draft = "",
        messages = emptyList(), authenticated = true, composerReady = false, streaming = false,
        currentModel = "", attachments = emptyList(), dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY, pageKind = "conversation",
    )

    @Test
    fun persistsConfirmedSpaConversationWithoutComposerOrPrivateSendReadiness() {
        assertEquals(snapshot.url, ChatGptWebSessionRestorer.confirmedConversationUrl(snapshot))
        val project = "https://chatgpt.com/g/g-p-project/c/fixture"
        assertEquals(project, ChatGptWebSessionRestorer.confirmedConversationUrl(
            snapshot.copy(url = "$project?source=sidebar#latest"),
        ))
        assertEquals(snapshot.url, ChatGptWebSessionRestorer.confirmedConversationUrl(
            snapshot.copy(streaming = true),
        ))
    }

    @Test
    fun rejectsUnconfirmedCachedLoginAndTemporaryOwnership() {
        listOf(
            snapshot.copy(authenticated = false), snapshot.copy(contentOnly = true),
            snapshot.copy(loginRequired = true), snapshot.copy(accessReason = "login_required"),
            snapshot.copy(pageKind = "auth"),
        ).forEach { assertNull(ChatGptWebSessionRestorer.confirmedConversationUrl(it)) }
        listOf("true", "", "unknown", "false&temporary-chat=true").forEach { flag ->
            assertNull(ChatGptWebSessionRestorer.confirmedConversationUrl(
                snapshot.copy(url = snapshot.url + "?temporary-chat=$flag"),
            ))
        }
        assertEquals(snapshot.url, ChatGptWebSessionRestorer.confirmedConversationUrl(
            snapshot.copy(url = snapshot.url + "?temporary-chat=false"),
        ))
    }

    @Test
    fun transientHomeProjectsAndForeignUrlsDoNotReplaceLastConversation() {
        listOf(
            "https://chatgpt.com/", "https://chatgpt.com/g/g-p-project/project",
            "https://chatgpt.com/auth/login", "https://other.invalid/c/fixture",
            "http://chatgpt.com/c/fixture", "https://user@chatgpt.com/c/fixture",
            "https://chatgpt.com:8443/c/fixture", "https://chatgpt.com/c/%2e%2e/auth/login",
            "https://chatgpt.com/c/fixture%2fother",
        ).forEach { url ->
            assertNull(url, ChatGptWebSessionRestorer.confirmedConversationUrl(snapshot.copy(url = url)))
        }
    }
}
