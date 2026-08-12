package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebBridgeReadinessPolicyTest {
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
