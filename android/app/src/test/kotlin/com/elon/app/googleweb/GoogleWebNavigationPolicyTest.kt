package com.elon.app.googleweb

import org.junit.Assert.assertFalse
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
}
