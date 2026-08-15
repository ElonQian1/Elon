package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatNavigationSessionRegistryTest {
    @Test
    fun routesACompleteGoogleProviderThroughTheCommonNavigationSession() {
        val calls = mutableListOf<String>()
        val registry = WebChatNavigationSessionRegistry(
            sessions = listOf(
                navigationSession(
                    providerId = WebChatProviderId.GOOGLE_WEB,
                    calls = calls,
                ),
            ),
            identity = { completeIdentity(it) },
        )

        val selected = requireNotNull(registry.session(WebChatProviderId.GOOGLE_WEB))
        selected.index()
        assertTrue(selected.refresh())
        assertTrue(selected.newConversation())
        assertTrue(selected.openConversation("/search/example"))
        assertTrue(selected.openProject("/project/example"))

        assertEquals(
            listOf("index", "refresh", "new", "conversation:/search/example", "project:/project/example"),
            calls,
        )
    }

    @Test
    fun routesProductionProvidersWithCapabilitiesBeyondNavigation() {
        WebChatProviderId.entries.forEach { providerId ->
            val registry = WebChatNavigationSessionRegistry(
                sessions = listOf(navigationSession(providerId)),
                identity = WebChatProviderRegistry::get,
            )

            requireNotNull(registry.session(providerId))
        }
    }

    @Test
    fun rejectsAProviderWhoseRuntimeAdapterLacksAnyRequiredNavigationCapability() {
        val incomplete = navigationSession(
            providerId = WebChatProviderId.GOOGLE_WEB,
            capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION -
                WebChatProviderCapability.PROJECT_LIST,
        )
        val registry = WebChatNavigationSessionRegistry(
            sessions = listOf(incomplete),
            identity = { completeIdentity(it) },
        )

        assertNull(registry.session(WebChatProviderId.GOOGLE_WEB))
    }

    @Test
    fun rejectsAnUnavailableProviderEvenWhenASessionWasRegistered() {
        val registry = WebChatNavigationSessionRegistry(
            sessions = listOf(navigationSession(WebChatProviderId.GOOGLE_WEB)),
            identity = { id ->
                completeIdentity(id).copy(available = false)
            },
        )

        assertNull(registry.session(WebChatProviderId.GOOGLE_WEB))
    }

    @Test
    fun rejectsASessionCapabilityThatTheProviderDoesNotDeclare() {
        val registry = WebChatNavigationSessionRegistry(
            sessions = listOf(
                navigationSession(
                    providerId = WebChatProviderId.GOOGLE_WEB,
                    capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION +
                        WebChatProviderCapability.REALTIME_VOICE,
                ),
            ),
            identity = { completeIdentity(it) },
        )

        assertNull(registry.session(WebChatProviderId.GOOGLE_WEB))
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsDuplicateProviderSessions() {
        WebChatNavigationSessionRegistry(
            sessions = listOf(
                navigationSession(WebChatProviderId.CHATGPT_WEB),
                navigationSession(WebChatProviderId.CHATGPT_WEB),
            ),
        )
    }

    private fun navigationSession(
        providerId: WebChatProviderId,
        capabilities: Set<WebChatProviderCapability> =
            WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION,
        calls: MutableList<String> = mutableListOf(),
    ) = WebChatNavigationSession(
        providerId = providerId,
        capabilities = capabilities,
        indexSource = {
            calls += "index"
            ChatGptWebConversationIndexState()
        },
        refreshSource = {
            calls += "refresh"
            true
        },
        newConversationSource = {
            calls += "new"
            true
        },
        openConversationSource = { path ->
            calls += "conversation:$path"
            true
        },
        openProjectSource = { path ->
            calls += "project:$path"
            true
        },
    )

    private fun completeIdentity(providerId: WebChatProviderId) = WebChatProviderIdentity(
        id = providerId,
        displayName = "Test provider",
        avatarResId = R.drawable.ic_web_ai_google_placeholder_avatar,
        available = true,
        capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION,
    )
}
