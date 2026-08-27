package com.elon.app.googleweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebNavigationPolicyTest {
    @Test
    fun acceptsOnlyOfficialGoogleAiModePages() {
        assertTrue(GoogleWebNavigationPolicy.supportsAiMode("https://www.google.com/aimode"))
        assertTrue(GoogleWebNavigationPolicy.supportsAiMode("https://www.google.com/webhp?aep=11"))
        assertTrue(GoogleWebNavigationPolicy.supportsAiMode("https://google.com/search?udm=50&q=test"))
        assertTrue(GoogleWebNavigationPolicy.supportsAiMode("https://www.google.com/search?aep=11&q=test"))
        assertFalse(GoogleWebNavigationPolicy.supportsAiMode("https://accounts.google.com/"))
        assertFalse(GoogleWebNavigationPolicy.supportsAiMode("https://google.com.evil.example/aimode"))
        assertFalse(GoogleWebNavigationPolicy.supportsAiMode("http://www.google.com/aimode"))
        assertFalse(GoogleWebNavigationPolicy.supportsAiMode("https://www.google.com/search?q=test"))
        assertFalse(GoogleWebNavigationPolicy.supportsAiMode("https://www.google.com/webhp"))
    }

    @Test
    fun stripsVolatileTrackingParametersFromRestorableUrls() {
        assertEquals(
            "https://www.google.com/search?q=hello%20world&udm=50&aep=11&hl=zh-CN&csuir=thread-123",
            GoogleWebNavigationPolicy.sanitizeNavigableUrl(
                "https://google.com/search?sei=volatile&q=hello%20world&udm=50&ved=tracking&aep=11&hl=zh-CN&csuir=thread-123&mstk=volatile",
            ),
        )
        assertEquals(
            "https://www.google.com/aimode",
            GoogleWebNavigationPolicy.sanitizeNavigableUrl(
                "https://www.google.com/aimode?sei=volatile",
            ),
        )
    }

    @Test
    fun distinguishesTransientPromptExecutionFromDurableConversations() {
        assertEquals(
            "https://www.google.com/search?q=first&udm=50&aep=11",
            GoogleWebNavigationPolicy.sanitizeNavigableUrl(
                "https://www.google.com/search?udm=50&aep=11&q=first&sei=volatile",
            ),
        )
        assertNull(GoogleWebNavigationPolicy.sanitizeConversationUrl(
            "https://www.google.com/search?udm=50&aep=11&q=first",
        ))
        assertNull(GoogleWebNavigationPolicy.sanitizeConversationUrl(
            "https://www.google.com/aimode",
        ))
        assertNull(GoogleWebNavigationPolicy.sanitizeConversationUrl(
            "https://www.google.com/search?udm=50&aep=11&q=first&csuir=",
        ))
        assertEquals(
            "https://www.google.com/search?q=first&udm=50&aep=11&csuir=thread-123",
            GoogleWebNavigationPolicy.sanitizeConversationUrl(
                "https://google.com/search?sei=volatile&udm=50&aep=11&q=first&csuir=thread-123",
            ),
        )
    }
}
