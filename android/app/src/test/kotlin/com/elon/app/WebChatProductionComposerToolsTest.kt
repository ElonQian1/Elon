package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionComposerToolsTest {
    @Test
    fun currentPrivateToolCatalogDoesNotWaitForTheComposer() {
        assertTrue(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
            WebChatProviderId.CHATGPT_WEB, sessionReady = false, state = document(),
        ))
        assertTrue(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
            WebChatProviderId.CHATGPT_WEB, sessionReady = true, state = document(),
        ))
    }

    @Test
    fun staleOrMissingDocumentCannotUseThePrivateCatalogEvenWithCachedReadyState() {
        listOf(null, document().copy(adapterCurrent = false)).forEach { state ->
            assertFalse(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
                WebChatProviderId.CHATGPT_WEB, sessionReady = true, state = state,
            ))
        }
    }

    @Test
    fun privateToolCatalogStillRejectsIdentityAndForeignPages() {
        listOf("https://chatgpt.com/auth/login", "https://accounts.google.com/", "https://example.com/",
            "http://chatgpt.com/", "https://chatgpt.com:444/", "https://user@chatgpt.com/").forEach { url ->
            assertFalse(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
                WebChatProviderId.CHATGPT_WEB, true, document().copy(pageUrl = url),
            ))
        }
        listOf("login", "auth", "challenge", "blocked").forEach { kind ->
            assertFalse(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
                WebChatProviderId.CHATGPT_WEB, true, document().copy(pageKind = kind),
            ))
        }
    }

    @Test
    fun nonChatGptProviderKeepsItsExistingSessionAdmission() {
        assertFalse(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
            WebChatProviderId.GOOGLE_WEB, false, document(),
        ))
        assertTrue(WebChatProductionQuickActionSyncPolicy.canReadCatalog(
            WebChatProviderId.GOOGLE_WEB, true, document(),
        ))
    }

    @Test
    fun staleToolOptionsCannotSupersedeTheCurrentUnfinishedRead() {
        assertTrue(WebChatProductionQuickActionSyncPolicy.canUseOptions(WebChatConsumerCommandStatus.SUCCEEDED))
        (WebChatConsumerCommandStatus.entries.filter { it != WebChatConsumerCommandStatus.SUCCEEDED } + null)
            .forEach { assertFalse(WebChatProductionQuickActionSyncPolicy.canUseOptions(it)) }
    }

    @Test
    fun parsesCachedToolsForTheProductionComposer() {
        val result = WebChatProductionComposerToolParser.parse(listOf(
            tool("search", "搜索", selected = true, selector = "chatgpt-tool:search"),
            tool("study", "学习", selected = false, selector = "chatgpt-tool:study"),
        ))

        assertEquals(listOf("search", "study"), result.map { it.id })
        assertTrue(result.first().selected)
        assertFalse(result.last().selected)
        assertEquals("web_search", result.first().semantic)
        assertEquals("chatgpt-tool:search", result.first().nativeSelector)
    }

    @Test
    fun ignoresInvalidAndDuplicateOptionsAndKeepsStableFallbackSelector() {
        val result = WebChatProductionComposerToolParser.parse(listOf(
            tool("canvas", "画布", selected = false, selector = ""),
            tool("canvas", "重复画布", selected = true, selector = "duplicate"),
            tool("missing_label", "", selected = false, selector = "invalid"),
        ))

        assertEquals(1, result.size)
        assertEquals("web-chat-composer-tool:canvas", result.single().nativeSelector)
        assertFalse(result.single().selected)
    }

    @Test
    fun returnsEmptyWhenTheOfficialPageHasNoToolSection() {
        assertTrue(WebChatProductionComposerToolParser.parse(emptyList()).isEmpty())
    }

    @Test
    fun retriesCompletedDiscoveryInsteadOfDeclaringAFeatureUnsupported() {
        assertEquals(
            WebChatProductionQuickActionSyncOutcome.KEEP_WAITING,
            WebChatProductionQuickActionSyncPolicy.resolve(
                WebChatConsumerCommandStatus.SUCCEEDED,
                attemptsExhausted = false,
                discoveryRound = 0,
                maxDiscoveryRounds = 2,
            ),
        )
        assertEquals(
            WebChatProductionQuickActionSyncOutcome.RETRY_DISCOVERY,
            WebChatProductionQuickActionSyncPolicy.resolve(
                WebChatConsumerCommandStatus.SUCCEEDED,
                attemptsExhausted = true,
                discoveryRound = 0,
                maxDiscoveryRounds = 2,
            ),
        )
        assertEquals(
            WebChatProductionQuickActionSyncOutcome.RETRY_LATER,
            WebChatProductionQuickActionSyncPolicy.resolve(
                WebChatConsumerCommandStatus.SUCCEEDED,
                attemptsExhausted = true,
                discoveryRound = 2,
                maxDiscoveryRounds = 2,
            ),
        )
    }

    @Test
    fun failedOrTimedOutToolSyncAsksForRetryInsteadOfBlamingTheOfficialProduct() {
        listOf(
            WebChatConsumerCommandStatus.FAILED,
            WebChatConsumerCommandStatus.TIMED_OUT,
            WebChatConsumerCommandStatus.PENDING,
        ).forEach { status ->
            assertEquals(
                WebChatProductionQuickActionSyncOutcome.RETRY_LATER,
                WebChatProductionQuickActionSyncPolicy.resolve(
                    status,
                    attemptsExhausted = true,
                    discoveryRound = 0,
                    maxDiscoveryRounds = 2,
                ),
            )
        }
    }

    @Test
    fun activeDictationCanFinishWhileTheOfficialComposerIsTransientlyUnavailable() {
        val activeDictation = WebChatConsumerState(
            streaming = false,
            dictationActive = true,
            composerSections = emptyMap(),
            pageKind = "home",
            pageUrl = "https://chatgpt.com/",
            features = emptyList(),
            commandRequests = emptyList(),
            adapterCurrent = true,
        )

        listOf("chatgpt_submit_dictation", "chatgpt_cancel_dictation").forEach { action ->
            assertTrue(WebChatProductionSessionCommandPolicy.canDispatch(
                action = action,
                sessionReady = false,
                consumerState = activeDictation,
            ))
            assertFalse(WebChatProductionSessionCommandPolicy.mayRecoverSession(action))
        }
        assertFalse(WebChatProductionSessionCommandPolicy.canDispatch(
            action = "chatgpt_start_dictation",
            sessionReady = false,
            consumerState = activeDictation,
        ))
        assertFalse(WebChatProductionSessionCommandPolicy.canDispatch(
            action = "chatgpt_cancel_dictation",
            sessionReady = false,
            consumerState = activeDictation.copy(adapterCurrent = false),
        ))
        assertTrue(WebChatProductionSessionCommandPolicy.mayRecoverSession("chatgpt_start_dictation"))
    }

    private fun document() = WebChatConsumerState(
        streaming = false, dictationActive = false, composerSections = emptyMap(), pageKind = "conversation",
        pageUrl = "https://chatgpt.com/c/test", features = emptyList(), commandRequests = emptyList(),
    )

    private fun tool(
        id: String,
        label: String,
        selected: Boolean,
        selector: String,
        semantic: String = if (id == "search") "web_search" else "",
    ) = WebChatConsumerOption(
        id = id,
        label = label,
        selected = selected,
        semantic = semantic,
        opensSubmenu = false,
        nativeSelector = selector,
    )
}
