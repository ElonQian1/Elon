package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionPrewarmAttemptsTest {
    @Test
    fun failedDiscoveryUsesBoundedBackoffWithoutSchedulingAnyTimer() {
        var now = 0L
        val attempts = WebChatProductionPrewarmAttempts { now }
        val key = key("model")
        for (delay in listOf(60_000L, 300_000L, 900_000L, 900_000L)) {
            assertTrue(attempts.eligible(key))
            attempts.started(key)
            now += delay - 1
            assertFalse(attempts.eligible(key))
            now += 1
            assertTrue(attempts.eligible(key))
        }
    }

    @Test
    fun confirmedObservationResetsOnlyThatCapability() {
        var now = 0L
        val attempts = WebChatProductionPrewarmAttempts { now }
        val models = key("model")
        val tools = key("tools")
        attempts.started(models)
        attempts.started(tools)
        attempts.confirmed(models)
        assertTrue(attempts.eligible(models))
        assertFalse(attempts.eligible(tools))

        attempts.started(models)
        now += 60_000L
        assertTrue(attempts.eligible(models))
    }

    @Test
    fun providerAndConversationControlScopesStayIndependent() {
        val attempts = WebChatProductionPrewarmAttempts { 0L }
        val models = key("model")
        val first = key("controls", "first")
        attempts.started(models)
        attempts.started(first)
        assertFalse(attempts.eligible(models))
        assertTrue(attempts.eligible(models.copy(providerId = WebChatProviderId.GOOGLE_WEB)))
        assertFalse(attempts.eligible(first))
        assertTrue(attempts.eligible(first.copy(pageKey = "second")))
    }

    @Test
    fun boundedConversationHistoryDoesNotEvictProviderCatalogFailures() {
        val attempts = WebChatProductionPrewarmAttempts { 0L }
        val models = key("model")
        attempts.started(models)
        repeat(200) { attempts.started(key("controls", "page-$it")) }
        assertFalse(attempts.eligible(models))
        assertTrue(attempts.eligible(key("controls", "page-0")))
        assertFalse(attempts.eligible(key("controls", "page-199")))
    }

    @Test
    fun clockCorrectionCannotLeaveDiscoveryPermanentlyBlocked() {
        var now = 100_000L
        val attempts = WebChatProductionPrewarmAttempts { now }
        val models = key("model")
        attempts.started(models)
        now = 0L
        assertTrue(attempts.eligible(models))
        attempts.started(models)
        assertFalse(attempts.eligible(models))
        now = 60_000L
        assertTrue(attempts.eligible(models))
    }

    private fun key(capability: String, page: String? = null) =
        WebChatProductionPrewarmAttempts.Key(WebChatProviderId.CHATGPT_WEB, capability, page)
}
