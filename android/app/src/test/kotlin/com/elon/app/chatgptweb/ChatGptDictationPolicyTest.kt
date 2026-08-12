package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptDictationPolicyTest {
    @Test
    fun enabledSemanticControlBridgesSnapshotCapabilityLag() {
        val manifest = manifest(control(enabled = true))

        assertNotNull(ChatGptDictationPolicy.resolve(manifest))
        assertTrue(ChatGptDictationPolicy.isAvailable(null, manifest))
        assertFalse(ChatGptDictationPolicy.isAvailable(null, manifest(control(enabled = false))))
        assertFalse(ChatGptDictationPolicy.isAvailable(null, null))
    }

    private fun manifest(control: ChatGptWebUiControl): ChatGptWebUiManifest = ChatGptWebUiManifest(
        version = 8,
        pageKind = "home",
        title = "ChatGPT",
        compatibility = "healthy",
        controls = listOf(control),
    )

    private fun control(enabled: Boolean): ChatGptWebUiControl = ChatGptWebUiControl(
        id = "control_dictation",
        semantic = ChatGptDictationPolicy.SEMANTIC,
        label = "开始听写",
        region = ChatGptWebUiRegion.COMPOSER,
        role = "button",
        enabled = enabled,
        selected = false,
    )
}
