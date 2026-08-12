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
        val toolStatePolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer_tool_state_policy.js",
        )
        val actionTargetPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_action_target_policy.js",
        )
        val dictationSessionPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_dictation_session_policy.js",
        )
        val modelLabelPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_model_label_policy.js",
        )
        val layoutAdapter = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_layout.js",
        )
        val core = readRepositoryFile("android/app/src/main/assets/chatgpt_web_adapter.js")
        val pageAdapter = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )

        val policyAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer_option_policy.js")
        val toolStateAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer_tool_state_policy.js")
        val actionTargetAsset = pageAdapter.indexOf("chatgpt_web_adapter_action_target_policy.js")
        val dictationSessionAsset = pageAdapter.indexOf("chatgpt_web_adapter_dictation_session_policy.js")
        val modelLabelAsset = pageAdapter.indexOf("chatgpt_web_adapter_model_label_policy.js")
        val composerAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer.js")
        assertTrue(modelLabelAsset >= 0)
        assertTrue(policyAsset > modelLabelAsset)
        assertTrue(toolStateAsset > policyAsset)
        assertTrue(actionTargetAsset > toolStateAsset)
        assertTrue(dictationSessionAsset > actionTargetAsset)
        assertTrue(composerAsset > dictationSessionAsset)
        assertTrue(core.contains("composerAdapter.capabilities(composer)"))
        assertTrue(core.contains("action === 'list_model_options'"))
        assertTrue(core.contains("action === 'list_composer_tools'"))
        assertTrue(adapter.contains("#upload-fast-tools-files"))
        assertTrue(adapter.contains("#composer-plus-btn"))
        assertTrue(adapter.contains("layout.findSemanticNode('attachment', 'composer')"))
        assertTrue(adapter.contains("scope.querySelectorAll('button, [role=\"button\"]')"))
        assertTrue(adapter.contains("layout.findSemanticNode('model', 'composer')"))
        assertTrue(adapter.contains("modelLabelPolicy.isModelLabel"))
        assertTrue(layoutAdapter.contains("const modelSignal = cleanText(signal + ' ' + labelOf(node, ''))"))
        assertTrue(layoutAdapter.contains("modelLabelPolicy.isModelLabel(modelSignal)"))
        assertTrue(modelLabelPolicy.contains("function isModelLabel"))
        assertTrue(adapter.contains("function emitVisibleNodeTouch(purpose, node, emitEvent)"))
        assertTrue(layoutAdapter.contains("function findSemanticNode(semantic, region)"))
        assertTrue(layoutAdapter.contains("function setNodeExpanded(node, expanded, emitEvent, result)"))
        assertTrue(layoutAdapter.contains("setNodeExpanded,"))
        assertTrue(layoutAdapter.contains("'control_disclosure_' + hash"))
        assertFalse(layoutAdapter.substringAfter("function setNodeExpanded").substringBefore("window.__elonChatGptLayout").contains("discover();"))
        assertTrue(layoutAdapter.contains("(!region || candidate.region === region)"))
        assertTrue(adapter.contains("composer_controls_snapshot"))
        assertTrue(adapter.contains("baseline: captureOptionBaseline()"))
        assertTrue(adapter.contains("actionTargetPolicy.signature(node)"))
        assertTrue(adapter.contains("isNewOrChangedOption(node, baseline)"))
        assertTrue(adapter.contains("web_touch_request"))
        assertTrue(adapter.contains("const reusable = lastOptions[section].filter"))
        assertTrue(adapter.contains("const alreadyOpen = collectOptions(section, null)"))
        assertTrue(adapter.contains("function isOptionVisible(node)"))
        assertTrue(adapter.contains("rect.top < window.innerHeight"))
        assertTrue(adapter.contains("actionTargetPolicy.actionPoint(node)"))
        assertTrue(adapter.contains("filter((option) => isOptionVisible(option.node))"))
        assertTrue(adapter.contains("!target || !isOptionVisible(target.node)"))
        assertTrue(adapter.contains("emitVisibleNodeTouch(purpose, target.node, emitEvent)"))
        assertTrue(adapter.contains("function optionSemantic(section, node, label)"))
        assertTrue(adapter.contains("const semantic = optionSemantic(section, node, label)"))
        assertTrue(adapter.contains("layout.findSemanticNode('web_search', 'composer')"))
        assertTrue(adapter.contains("composerToolStatePolicy.optionSelected"))
        assertTrue(layoutAdapter.contains("composerToolStatePolicy.semantic"))
        assertTrue(layoutAdapter.contains("composerToolStatePolicy.controlSelected"))
        assertTrue(toolStatePolicy.contains("function isWebSearchSignal"))
        assertTrue(toolStatePolicy.contains("function optionSelected"))
        assertTrue(adapter.contains("opensSubmenu: opensSubmenu(node)"))
        assertTrue(adapter.contains("open_model_submenu"))
        assertTrue(adapter.contains("layout.setNodeExpanded(target.node, true"))
        assertTrue(adapter.contains("collectRequestedOptions(section, composer, emitEvent, () => {})"))
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
        assertTrue(optionPolicy.contains("root && root.__elonChatGptModelLabelPolicy"))
        assertTrue(optionPolicy.contains("modelLabelPolicy.isModelLabel(label)"))
        assertTrue(optionPolicy.contains("isForeignMenuLabel"))
        assertTrue(optionPolicy.contains("return []"))
        assertTrue(adapter.contains("readAttachments"))
        assertTrue(adapter.contains("removeAttachment"))
        assertTrue(adapter.contains("dictationActive"))
        assertTrue(adapter.contains("layout.findSemanticNode('dictation', 'composer')"))
        assertTrue(adapter.contains("layout.requestSemanticTouch('dictation', 'start_dictation'"))
        assertTrue(adapter.contains("dictationSessionPolicy.find"))
        assertTrue(adapter.contains("dictationSessionPolicy.active"))
        assertTrue(dictationSessionPolicy.contains("cancel dictation"))
        assertTrue(dictationSessionPolicy.contains("submit dictation"))
        assertTrue(dictationSessionPolicy.contains("取消听写"))
        assertTrue(dictationSessionPolicy.contains("提交听写"))
        assertTrue(dictationSessionPolicy.contains("composerPresent"))
        assertTrue(core.contains("action === 'cancel_dictation'"))
        assertTrue(core.contains("action === 'submit_dictation'"))
        assertFalse(adapter.contains("input.click()"))
        assertTrue(adapter.contains("选项已过期"))
        assertTrue(actionTargetPolicy.contains("documentRef.elementFromPoint"))
        assertTrue(actionTargetPolicy.contains("style.pointerEvents === 'none'"))
        assertTrue(actionTargetPolicy.contains("clipsChildren(style)"))
        listOf(
            adapter,
            optionPolicy,
            toolStatePolicy,
            actionTargetPolicy,
            dictationSessionPolicy,
        ).forEach { source ->
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
        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeRealtimeVoice\""))
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
        assertTrue(voiceController.contains("ChatGptDictationPolicy.isAvailable(value, manifest)"))
        val realtimeVoiceController = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeRealtimeVoiceController.kt",
        )
        val realtimeVoicePolicy = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptRealtimeVoicePolicy.kt",
        )
        assertTrue(realtimeVoicePolicy.contains("const val SEMANTIC = \"voice_mode\""))
        assertTrue(realtimeVoiceController.contains("ChatGptNativeNavigationSelector.REALTIME_VOICE"))
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
