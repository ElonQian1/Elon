package com.elon.app.googleweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GoogleWebConversationResumePolicyTest {
    private val transient = "https://www.google.com/search?q=first&udm=50&aep=11"
    private val stable = "$transient&csuir=thread-123"

    @Test
    fun startupNeverReplaysATransientPromptUrl() {
        assertNull(GoogleWebConversationResumePolicy.persistableUrl(transient))
        assertEquals(
            GoogleWebNavigationPolicy.START_URL,
            GoogleWebConversationResumePolicy.startupUrl(transient),
        )
        assertEquals(stable, GoogleWebConversationResumePolicy.startupUrl(stable))
    }

    @Test
    fun reloadNeverReexecutesThePromptInTheCurrentUrl() {
        assertEquals(
            GoogleWebNavigationPolicy.START_URL,
            GoogleWebConversationResumePolicy.reloadUrl(transient, stable),
        )
        assertEquals(stable, GoogleWebConversationResumePolicy.reloadUrl(stable, null))
        assertEquals(stable, GoogleWebConversationResumePolicy.reloadUrl("about:blank", stable))
    }

    @Test
    fun officialFallbackUsesOnlyStableConversationIdentity() {
        assertEquals(
            GoogleWebNavigationPolicy.START_URL,
            GoogleWebConversationResumePolicy.officialUrl(null, transient),
        )
        assertEquals(stable, GoogleWebConversationResumePolicy.officialUrl(stable, transient))
    }
}
