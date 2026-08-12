package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebSessionStateStoreTest {
    @Test
    fun keepsOnlySafeRestorableChatGptPathsWithoutQueriesOrFragments() {
        assertEquals(
            "https://chatgpt.com/c/conversation_123",
            ChatGptWebSessionStateStore.normalizeRestorableUrl(
                "https://chatgpt.com/c/conversation_123?temporary=value#latest",
            ),
        )
        assertEquals(
            "https://chatgpt.com/projects/demo",
            ChatGptWebSessionStateStore.normalizeRestorableUrl(
                "https://chatgpt.com/projects/demo",
            ),
        )
        assertEquals(
            ChatGptWebNavigationPolicy.START_URL,
            ChatGptWebSessionStateStore.normalizeRestorableUrl("https://chatgpt.com/"),
        )
        listOf(
            "/scheduled",
            "/plugins",
            "/studymode",
            "/health",
            "/finance",
            "/finances",
            "/work",
        )
            .forEach { path ->
                assertEquals(
                    "https://chatgpt.com$path",
                    ChatGptWebSessionStateStore.normalizeRestorableUrl(
                        "https://chatgpt.com$path?private=value#section",
                    ),
                )
            }
    }

    @Test
    fun rejectsAuthenticationExternalAndTraversalUrls() {
        listOf(
            "https://chatgpt.com/auth/login",
            "https://chatgpt.com/cdn-cgi/challenge",
            "https://accounts.google.com/signin",
            "http://chatgpt.com/c/demo",
            "https://user@chatgpt.com/c/demo",
            "https://chatgpt.com:8443/c/demo",
            "https://chatgpt.com/c/../auth/login",
            "https://chatgpt.com/c/%2e%2e/auth/login",
            "https://chatgpt.com/backend-api/conversation",
            "https://chatgpt.com/tasks-legacy",
            "https://chatgpt.com/gpts-legacy",
        ).forEach { url ->
            assertNull(url, ChatGptWebSessionStateStore.normalizeRestorableUrl(url))
        }
    }
}
