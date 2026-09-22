package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ChatGptWebAccountConnectionTest {
    private fun snapshot() = ChatGptWebSnapshot(
        title = "", url = "https://chatgpt.com/", draft = "", messages = emptyList(),
        authenticated = true, accountConfirmed = true, composerReady = false, streaming = false, currentModel = "",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
    )

    @Test fun authenticatedEvidenceDoesNotRequireComposer() {
        assertEquals(ChatGptWebAccountConnection.State.CONNECTED,
            ChatGptWebAccountConnection.observedState(snapshot()))
    }

    @Test fun missingDomAndCachedHistoryCannotDeclareLoginOrLogout() {
        assertNull(ChatGptWebAccountConnection.observedState(snapshot().copy(accountConfirmed = false, composerReady = true)))
        assertNull(ChatGptWebAccountConnection.observedState(snapshot().copy(url = "https://chatgpt.com/auth/login")))
        assertNull(ChatGptWebAccountConnection.observedState(snapshot().copy(contentOnly = true)))
        assertNull(ChatGptWebAccountConnection.observedState(snapshot().copy(authenticated = false)))
        assertNull(ChatGptWebAccountConnection.observedState(snapshot().copy(
            authenticated = false, loginRequired = true, pageKind = "challenge")))
    }

    @Test fun foreignOriginsAndPortsCannotSetConnected() {
        listOf("http://chatgpt.com/", "https://example.com/", "https://chatgpt.com.evil.test/",
            "https://user@chatgpt.com/", "https://chatgpt.com:8443/").forEach {
            assertNull(it, ChatGptWebAccountConnection.observedState(snapshot().copy(url = it)))
        }
    }

    @Test fun observedLoginAndExplicitLogoutSurviveNewReaderButNotMissingDom() {
        val context = RuntimeEnvironment.getApplication()
        val connection = ChatGptWebAccountConnection(context)
        connection.observe(ChatGptWebEvent.Snapshot(snapshot()))
        connection.observe(ChatGptWebEvent.Snapshot(snapshot().copy(authenticated = false)))
        assertEquals(ChatGptWebAccountConnection.State.CONNECTED, ChatGptWebAccountConnection(context).state())
        connection.observe(ChatGptWebEvent.Snapshot(snapshot().copy(authenticated = false,
            loginRequired = true, pageKind = "auth")))
        assertEquals(ChatGptWebAccountConnection.State.LOGIN_REQUIRED, connection.state())
        val stored = context.getSharedPreferences("chatgpt_account_connection_v1", 0).all
        assertEquals(setOf("state"), stored.keys)
    }

    @Test fun loginReturnOnlyCompletesOnceAfterAuthenticatedEvidence() {
        val completion = ChatGptWebLoginCompletion(true)
        assertFalse(completion.accept(snapshot().copy(authenticated = false, composerReady = true)))
        assertTrue(completion.accept(snapshot()))
        assertFalse(completion.accept(snapshot()))
        assertFalse(ChatGptWebLoginCompletion(false).accept(snapshot()))
    }

    @Test fun loginIntentDoesNotLaunchVoiceOrSendAction() {
        val intent = ChatGptWebLoginReturn.intent(RuntimeEnvironment.getApplication())
        assertEquals(ChatGptWebOfficialActivity::class.java.name, intent.component?.className)
        assertTrue(intent.getBooleanExtra("chatgpt_return_after_login", false))
        assertTrue(intent.extras!!.keySet().none { it.contains("voice") || it.contains("send") })
    }

    @Test fun protocolRequiresExplicitConfirmationInsteadOfLegacyAuthenticatedHint() {
        val legacy = ChatGptWebProtocol.parse("""{"schema":"yilong.ai.ui.v1","event":{"type":"message_snapshot","url":"https://chatgpt.com/","authenticated":true}}""")
            as ChatGptWebEvent.Snapshot
        val confirmed = ChatGptWebProtocol.parse("""{"schema":"yilong.ai.ui.v1","event":{"type":"message_snapshot","url":"https://chatgpt.com/","authenticated":true,"accountConfirmed":true}}""")
            as ChatGptWebEvent.Snapshot
        assertNull(ChatGptWebAccountConnection.observedState(legacy.value))
        assertEquals(ChatGptWebAccountConnection.State.CONNECTED,
            ChatGptWebAccountConnection.observedState(confirmed.value))
    }
}
