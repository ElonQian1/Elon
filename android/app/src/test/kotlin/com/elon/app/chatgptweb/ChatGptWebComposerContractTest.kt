package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebComposerContractTest {
    @Test
    fun composerAdapterDiscoversOfficialControlsWithoutReadingProviderTraffic() {
        val adapter = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer.js",
        )
        val optionPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer_option_policy.js",
        )
        val core = readRepositoryFile("android/app/src/main/assets/chatgpt_web_adapter.js")
        val pageAdapter = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )

        val policyAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer_option_policy.js")
        val composerAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer.js")
        assertTrue(policyAsset >= 0)
        assertTrue(composerAsset > policyAsset)
        assertTrue(core.contains("composerAdapter.capabilities(composer)"))
        assertTrue(core.contains("action === 'list_model_options'"))
        assertTrue(core.contains("action === 'list_composer_tools'"))
        assertTrue(adapter.contains("#upload-fast-tools-files"))
        assertTrue(adapter.contains("#composer-plus-btn"))
        assertTrue(adapter.contains("composer_controls_snapshot"))
        assertTrue(adapter.contains("baseline: new Set(visibleOptionNodes())"))
        assertTrue(adapter.contains("!baseline.has(node)"))
        assertTrue(adapter.contains("web_touch_request"))
        assertTrue(adapter.contains("const reusable = lastOptions[section].filter"))
        assertTrue(adapter.contains("const alreadyOpen = collectOptions(section, null)"))
        assertTrue(adapter.contains("rect.left < window.innerWidth"))
        assertTrue(adapter.contains("function optionSemantic(section, node, label)"))
        assertTrue(adapter.contains("semantic: optionSemantic(section, node, label)"))
        assertTrue(adapter.contains("opensSubmenu: opensSubmenu(node)"))
        assertTrue(adapter.contains("open_model_submenu"))
        assertTrue(adapter.contains("open_composer_tools_submenu"))
        assertTrue(adapter.contains("complete: result"))
        assertTrue(adapter.contains("settlePendingOptions(section, true, '')"))
        assertTrue(adapter.contains("previous.complete(previous.action, false"))
        assertTrue(adapter.contains("!pending.syntheticRetried"))
        assertTrue(adapter.contains("pending.trigger.click()"))
        assertTrue(adapter.contains("if (!target.opensSubmenu) {"))
        assertTrue(adapter.contains("window.setTimeout(scheduleSnapshot, 240)"))
        assertTrue(adapter.contains("deep_research"))
        assertTrue(adapter.contains("attachment_camera"))
        assertTrue(adapter.contains("web_search"))
        assertFalse(adapter.contains("documents?"))
        assertFalse(adapter.contains("|文档|"))
        assertTrue(adapter.contains("dismiss_composer_menu"))
        assertTrue(adapter.contains("optionPolicy.filter(section, candidates)"))
        assertTrue(optionPolicy.contains("isForeignMenuLabel"))
        assertTrue(optionPolicy.contains("return []"))
        assertTrue(adapter.contains("readAttachments"))
        assertTrue(adapter.contains("removeAttachment"))
        assertTrue(adapter.contains("dictationActive"))
        assertTrue(adapter.contains("cancel dictation"))
        assertTrue(adapter.contains("submit dictation"))
        assertTrue(adapter.contains("取消听写"))
        assertTrue(adapter.contains("提交听写"))
        assertTrue(core.contains("action === 'cancel_dictation'"))
        assertTrue(core.contains("action === 'submit_dictation'"))
        assertFalse(adapter.contains("input.click()"))
        assertTrue(adapter.contains("选项已过期"))
        listOf(adapter, optionPolicy).forEach { source ->
            listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
                assertFalse("composer adapter must not contain $it", source.contains(it))
            }
        }
    }

    @Test
    fun nativeAttachmentFlowUsesTheSystemPickerAndCurrentWebViewCallback() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt",
        )
        val chooser = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebFileChooserController.kt",
        )
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")

        assertTrue(activity.contains("override fun onShowFileChooser("))
        assertTrue(activity.contains("fileChooserController.show("))
        assertTrue(activity.contains("allowContentAccess = true"))
        assertTrue(chooser.contains("Intent.ACTION_OPEN_DOCUMENT"))
        assertTrue(chooser.contains("MediaStore.ACTION_IMAGE_CAPTURE"))
        assertTrue(chooser.contains("FileProvider.getUriForFile"))
        assertTrue(chooser.contains("FileChooserParams.parseResult"))
        assertTrue(chooser.contains("FLAG_GRANT_READ_URI_PERMISSION"))
        assertTrue(chooser.contains("supportsEnhancedMode(webView.url)"))
        assertFalse(chooser.contains("getCookie("))
        assertFalse(chooser.contains("readBytes("))
        assertFalse(manifest.contains("android.permission.READ_EXTERNAL_STORAGE"))
        assertFalse(manifest.contains("android.permission.MANAGE_EXTERNAL_STORAGE"))
    }

    @Test
    fun nativeComposerControlsRemainCapabilityGated() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeComposerToolsController.kt",
        )
        val layout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_chatgpt_web_test.xml",
        )
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")

        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeModel\""))
        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeAttachment\""))
        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeTools\""))
        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeDictation\""))
        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeAttachments\""))
        assertTrue(controller.contains("ChatGptWebCapabilityId.MODEL_SELECTOR"))
        assertTrue(controller.contains("ChatGptWebCapabilityId.ATTACHMENTS"))
        assertTrue(controller.contains("ChatGptWebCapabilityId.COMPOSER_TOOLS"))
        assertTrue(controller.contains("ChatGptWebComposerOptionSemantics.isAttachment"))
        assertTrue(controller.contains("usesSingleChoice(section, options)"))
        assertTrue(controller.contains("pendingSection = if (option.opensSubmenu) section else null"))
        assertFalse(controller.contains("ATTACHMENT_LABELS"))
        assertTrue(controller.contains("bridgeReady && capabilities.supports"))
        val voiceController = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeVoiceController.kt",
        )
        val permissionController = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAudioPermissionController.kt",
        )
        assertTrue(voiceController.contains("ChatGptWebCapabilityId.DICTATION"))
        assertTrue(permissionController.contains("request.origin.host == \"chatgpt.com\""))
        assertTrue(permissionController.contains("RESOURCE_AUDIO_CAPTURE"))
        assertTrue(manifest.contains("android.permission.RECORD_AUDIO"))
        assertTrue(manifest.contains("android.permission.MODIFY_AUDIO_SETTINGS"))
    }

    @Test
    fun trustedWebTouchesAreWhitelistedAndStayInsideTheCurrentWebView() {
        val dispatcher = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTouchDispatcher.kt",
        )
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt",
        )

        assertTrue(dispatcher.contains("request.purpose in ALLOWED_PURPOSES"))
        assertTrue(dispatcher.contains("supportsEnhancedMode(webView.url)"))
        assertTrue(dispatcher.contains("webView.dispatchTouchEvent(down)"))
        assertTrue(dispatcher.contains("webView.dispatchTouchEvent(up)"))
        assertTrue(dispatcher.contains("open_model_submenu"))
        assertTrue(dispatcher.contains("open_composer_tools_submenu"))
        assertFalse(dispatcher.contains("Instrumentation"))
        assertFalse(dispatcher.contains("AccessibilityService"))
        assertTrue(activity.contains("pageAdapter::collectModelOptions"))
        assertTrue(activity.contains("pageAdapter::collectComposerTools"))
        assertTrue(activity.contains("openOfficialComposerOptions(\"model\")"))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
