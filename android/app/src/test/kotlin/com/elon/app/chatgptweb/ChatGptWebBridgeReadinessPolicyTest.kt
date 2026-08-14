package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebBridgeReadinessPolicyTest {
    @Test
    fun pageFinishKeepsTheCurrentAdapterReady() {
        val session = WebBridgeDocumentSession { "doc_1_current" }
        val loading = session.beginPage()
        val current = session.accept(loading.documentToken)!!

        assertEquals(
            ChatGptWebPageAdapter.State.READY,
            ChatGptWebBridgeReadinessPolicy.stateAfterPageReady(
                listenerInstalled = true,
                enhancedModeSupported = true,
                document = current,
            ),
        )
    }

    @Test
    fun pageFinishWaitsWhenTheCurrentDocumentHasNotConnected() {
        val loading = WebBridgeDocumentSession { "doc_1_loading" }.beginPage()

        assertEquals(
            ChatGptWebPageAdapter.State.CONNECTING,
            ChatGptWebBridgeReadinessPolicy.stateAfterPageReady(
                listenerInstalled = true,
                enhancedModeSupported = true,
                document = loading,
            ),
        )
    }

    @Test
    fun pageFinishStillHonorsUnsupportedAndWebOnlySurfaces() {
        val document = WebBridgeDocumentSession.Snapshot(0, 0, "")

        assertEquals(
            ChatGptWebPageAdapter.State.WEB_ONLY,
            ChatGptWebBridgeReadinessPolicy.stateAfterPageReady(
                listenerInstalled = true,
                enhancedModeSupported = false,
                document = document,
            ),
        )
        assertEquals(
            ChatGptWebPageAdapter.State.UNSUPPORTED,
            ChatGptWebBridgeReadinessPolicy.stateAfterPageReady(
                listenerInstalled = false,
                enhancedModeSupported = true,
                document = document,
            ),
        )
    }

    @Test
    fun authenticatedDocumentCanRecoverFromAnOverlayManifest() {
        assertTrue(ChatGptWebBridgeReadinessPolicy.canRestoreFromManifest(
            snapshot(authenticated = true),
            manifest(pageKind = "home"),
        ))
    }

    @Test
    fun loginAndUnauthenticatedStatesCannotRecoverFromManifestAlone() {
        assertFalse(ChatGptWebBridgeReadinessPolicy.canRestoreFromManifest(
            snapshot(authenticated = false),
            manifest(pageKind = "home"),
        ))
        assertFalse(ChatGptWebBridgeReadinessPolicy.canRestoreFromManifest(
            snapshot(authenticated = true, loginRequired = true),
            manifest(pageKind = "home"),
        ))
        assertFalse(ChatGptWebBridgeReadinessPolicy.canRestoreFromManifest(
            snapshot(authenticated = true),
            manifest(pageKind = "auth"),
        ))
        assertFalse(ChatGptWebBridgeReadinessPolicy.canRestoreFromManifest(
            null,
            manifest(pageKind = "home"),
        ))
    }

    private fun snapshot(
        authenticated: Boolean,
        loginRequired: Boolean = false,
    ) = ChatGptWebSnapshot(
        title = "ChatGPT",
        url = "https://chatgpt.com/",
        draft = "",
        messages = emptyList(),
        authenticated = authenticated,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "home",
        loginRequired = loginRequired,
    )

    private fun manifest(pageKind: String) = ChatGptWebUiManifest(
        version = 8,
        pageKind = pageKind,
        title = "ChatGPT",
        compatibility = "healthy",
        controls = emptyList(),
    )
}
