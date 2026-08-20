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
        val submenuAdapter = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer_submenu.js",
        )
        val toolStatePolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer_tool_state_policy.js",
        )
        val toolSelection = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer_tool_selection.js",
        )
        val actionTargetPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_action_target_policy.js",
        )
        val attachmentPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_attachment_policy.js",
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
        val submenuAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer_submenu.js")
        val toolStateAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer_tool_state_policy.js")
        val toolSelectionAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer_tool_selection.js")
        val actionTargetAsset = pageAdapter.indexOf("chatgpt_web_adapter_action_target_policy.js")
        val dictationSessionAsset = pageAdapter.indexOf("chatgpt_web_adapter_dictation_session_policy.js")
        val modelLabelAsset = pageAdapter.indexOf("chatgpt_web_adapter_model_label_policy.js")
        val composerAsset = pageAdapter.indexOf("chatgpt_web_adapter_composer.js")
        assertTrue(modelLabelAsset >= 0)
        assertTrue(policyAsset > modelLabelAsset)
        assertTrue(submenuAsset > policyAsset)
        assertTrue(toolStateAsset > submenuAsset)
        assertTrue(toolSelectionAsset > toolStateAsset)
        assertTrue(actionTargetAsset > toolSelectionAsset)
        assertTrue(dictationSessionAsset > actionTargetAsset)
        assertTrue(composerAsset > dictationSessionAsset)
        assertTrue(core.contains("composerAdapter.capabilities(composer)"))
        assertTrue(core.contains("action === 'list_model_options'"))
        assertTrue(core.contains("action === 'list_composer_tools'"))
        assertTrue(core.contains("function waitForStableSendButton"))
        assertTrue(core.contains("function waitForSendAccepted"))
        assertTrue(core.contains("官方网页已确认发送"))
        assertTrue(adapter.contains("#upload-fast-tools-files"))
        assertTrue(adapter.contains("#composer-plus-btn"))
        assertTrue(adapter.contains("layout.findSemanticNode('attachment', 'composer')"))
        assertTrue(adapter.contains("scope.querySelectorAll('button, [role=\"button\"]')"))
        assertTrue(adapter.contains("layout.findSemanticNode('model', 'composer')"))
        assertTrue(adapter.contains("[data-testid=\"model-switcher\"]"))
        assertTrue(adapter.contains("button.getAttribute('data-model-title')"))
        assertTrue(adapter.contains("modelLabelPolicy.isModelLabel"))
        assertTrue(layoutAdapter.contains("const modelSignal = cleanText(signal + ' ' + labelOf(node, ''))"))
        assertTrue(layoutAdapter.contains("modelLabelPolicy.isModelControl({ label: labelOf(node, ''), signal: modelSignal })"))
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
        assertFalse(adapter.contains("const reusable = lastOptions[section].filter"))
        assertTrue(adapter.contains("trigger.getAttribute('aria-expanded') === 'true'"))
        assertTrue(adapter.contains("? collectOptions(section, null)"))
        assertTrue(adapter.contains("function isOptionVisible(node)"))
        assertTrue(adapter.contains("rect.top < window.innerHeight"))
        assertTrue(adapter.contains("actionTargetPolicy.actionPoint(node)"))
        assertFalse(adapter.contains("filter((option) => isOptionVisible(option.node))"))
        assertTrue(adapter.contains("if (!target)"))
        assertTrue(adapter.contains("if (!isOptionVisible(target.node))"))
        assertTrue(adapter.contains("emitVisibleNodeTouch(purpose, target.node, emitEvent)"))
        assertTrue(adapter.contains("function optionSemantic(section, node, label)"))
        assertTrue(adapter.contains("const semantic = optionSemantic(section, node, label)"))
        assertTrue(adapter.contains("layout.findSemanticNode(semantic, 'composer')"))
        assertTrue(adapter.contains("composerToolStatePolicy.optionSelected"))
        assertTrue(layoutAdapter.contains("composerToolStatePolicy.semantic"))
        assertTrue(layoutAdapter.contains("composerToolStatePolicy.controlSelected"))
        assertTrue(toolStatePolicy.contains("function isWebSearchSignal"))
        assertTrue(toolStatePolicy.contains("function isImageGenerationSignal"))
        assertTrue(toolStatePolicy.contains("function optionSelected"))
        assertTrue(toolStatePolicy.contains("function directSelection"))
        assertTrue(toolStatePolicy.contains("function createSelectionTracker"))
        assertTrue(adapter.contains("const composerToolSelection = composerToolStatePolicy"))
        assertTrue(adapter.contains("composerToolSelection.value(semantic, liveActiveInComposer)"))
        assertTrue(adapter.contains("composerToolSelection.observe('web_search', desiredSelected)"))
        assertTrue(toolSelection.contains("function verifyInMenu"))
        assertTrue(toolSelection.contains("target.directStateKnown"))
        assertTrue(toolSelection.contains("touchAttempt >= MAX_TOUCH_ATTEMPTS"))
        assertFalse(adapter.contains("verify_composer_tool"))
        assertFalse(adapter.contains("select_composer_tool_retry"))
        assertTrue(adapter.contains("opensSubmenu: opensSubmenu(node)"))
        assertTrue(adapter.contains("parentOption: parentOption ?"))
        assertTrue(adapter.contains("open_model_submenu"))
        assertFalse(adapter.contains("layout.setNodeExpanded(target.node, true"))
        assertTrue(adapter.contains("if (!emitVisibleNodeTouch(purpose, target.node, emitEvent))"))
        assertTrue(adapter.contains("}, 220)"))
        assertTrue(adapter.contains("collectRequestedOptions(section, composer, emitEvent, () => {})"))
        assertTrue(adapter.contains("submenuRecovery.recover"))
        assertTrue(adapter.contains("function waitForOptionsMatching"))
        assertTrue(submenuAdapter.contains("function withoutKnownOptionIds"))
        assertTrue(submenuAdapter.contains("function containsNestedInteractiveControl"))
        assertTrue(submenuAdapter.contains("function createRecovery"))
        assertTrue(adapter.contains("rootOptionIds: new Set(lastOptions[section].map"))
        assertTrue(adapter.contains("composerSubmenu.withoutKnownOptionIds(options, pending.rootOptionIds)"))
        assertTrue(adapter.contains("if (target.parentOption)"))
        assertTrue(submenuAdapter.contains("const parent = matchingOption(rootOptions, parentIdentity)"))
        assertTrue(submenuAdapter.contains("const recovered = matchingOption(nestedChildren, target)"))
        assertTrue(adapter.contains("open_composer_tools_submenu"))
        assertTrue(adapter.contains("complete: result"))
        assertTrue(adapter.contains("settlePendingOptions(section, true, '')"))
        assertTrue(adapter.contains("previous.complete(previous.action, false"))
        assertTrue(adapter.contains("!pending.syntheticRetried"))
        assertTrue(adapter.contains("pending.trigger.click()"))
        assertTrue(adapter.contains("if (!target.opensSubmenu) {"))
        assertTrue(submenuAdapter.contains("deps.schedule(scheduleSnapshot, 240)"))
        assertTrue(toolSelection.contains("const MAX_OBSERVATION_ATTEMPTS = 24"))
        assertTrue(toolSelection.contains("const REQUIRED_CONFIRMATIONS = 4"))
        assertTrue(toolSelection.contains("function completeWhenObserved"))
        assertTrue(toolSelection.contains("const observedSelection = !menuSettled && optionSelection.known"))
        assertTrue(toolSelection.contains("observedSelection.selected === context.desiredSelected"))
        assertTrue(toolSelection.contains("nextConfirmations >= REQUIRED_CONFIRMATIONS"))
        assertTrue(toolSelection.contains("官网网页搜索状态未发生预期变化"))
        assertTrue(adapter.contains("deep_research"))
        assertTrue(adapter.contains("attachment_camera"))
        assertTrue(adapter.contains("web_search"))
        assertFalse(adapter.contains("documents?"))
        assertFalse(adapter.contains("|文档|"))
        assertTrue(adapter.contains("dismiss_composer_menu"))
        assertTrue(adapter.contains("optionPolicy.filter(section, candidates)"))
        assertTrue(optionPolicy.contains("root && root.__elonChatGptModelLabelPolicy"))
        assertTrue(optionPolicy.contains("modelLabelPolicy.isModelLabel(label)"))
        assertTrue(optionPolicy.contains("capabilit(?:y|ies)"))
        assertTrue(optionPolicy.contains("isForeignMenuLabel"))
        assertTrue(optionPolicy.contains("return []"))
        assertTrue(adapter.contains("readAttachments"))
        assertTrue(adapter.contains("removeAttachment"))
        assertTrue(adapter.contains("attachmentPolicy.isRemoveActionLabel"))
        assertTrue(adapter.contains("attachmentPolicy.invokeRemoveAction"))
        assertTrue(adapter.indexOf("attachmentPolicy.invokeRemoveAction") < adapter.indexOf("emitTouchRequest('remove_attachment'"))
        assertTrue(adapter.contains("button, [role=\"button\"]"))
        assertTrue(attachmentPolicy.contains("移除|删除"))
        assertTrue(attachmentPolicy.contains("remove|delete"))
        assertTrue(attachmentPolicy.contains("isRemoveActionLabel(label)"))
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
        assertTrue(adapter.contains("requestAttachmentUpload(result)"))
        assertTrue(adapter.contains("input.click()"))
        assertTrue(core.contains("action === 'request_attachment_upload'"))
        assertTrue(adapter.contains("选项已过期"))
        assertTrue(actionTargetPolicy.contains("documentRef.elementFromPoint"))
        assertTrue(actionTargetPolicy.contains("style.pointerEvents === 'none'"))
        assertTrue(actionTargetPolicy.contains("clipsChildren(style)"))
        listOf(
            adapter,
            optionPolicy,
            submenuAdapter,
            toolStatePolicy,
            actionTargetPolicy,
            attachmentPolicy,
            dictationSessionPolicy,
        ).forEach { source ->
            listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
                assertFalse("composer adapter must not contain $it", source.contains(it))
            }
        }
    }

    @Test
    fun attachmentFlowUsesTheSystemPickerAndCurrentProductionWebViewCallback() {
        val official = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebOfficialActivity.kt",
        )
        val background = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val backgroundWebView = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundWebViewFactory.kt",
        )
        val chooser = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebFileChooserController.kt",
        )
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")

        listOf(official, backgroundWebView).forEach { host ->
            assertTrue(host.contains("override fun onShowFileChooser("))
            assertTrue(host.contains("allowContentAccess = true"))
        }
        assertTrue(official.contains("fileChooserController.show("))
        assertTrue(background.contains("val values = queuedUploadUris"))
        assertTrue(background.contains("queuedUploadUris = emptyList()"))
        assertTrue(background.contains("callback.onReceiveValue("))
        assertFalse(background.contains("fileChooserController.show("))
        assertTrue(chooser.contains("Intent.ACTION_OPEN_DOCUMENT"))
        assertTrue(chooser.contains("Intent.ACTION_GET_CONTENT"))
        assertTrue(chooser.contains("com.google.android.documentsui"))
        assertTrue(chooser.contains("MediaStore.ACTION_IMAGE_CAPTURE"))
        assertTrue(chooser.contains("FileProvider.getUriForFile"))
        assertTrue(chooser.contains("ChatGptWebFileSelectionResult.parse"))
        assertTrue(chooser.contains("FLAG_GRANT_READ_URI_PERMISSION"))
        assertTrue(chooser.contains("supportsEnhancedMode(webView.url)"))
        assertFalse(chooser.contains("getCookie("))
        assertFalse(chooser.contains("readBytes("))
        assertFalse(manifest.contains("android.permission.READ_EXTERNAL_STORAGE"))
        assertFalse(manifest.contains("android.permission.MANAGE_EXTERNAL_STORAGE"))
    }

    @Test
    fun productionComposerControlsRemainCapabilityGated() {
        val coordinator = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt",
        )
        val quickActions = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionQuickComposerActions.kt",
        )
        val background = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val optionRequests = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptComposerOptionRequestCoordinator.kt",
        )
        val optionInteraction = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptComposerOptionInteraction.kt",
        )
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")

        assertTrue(quickActions.contains("WebChatProviderCapability.COMPOSER_TOOLS"))
        assertTrue(coordinator.contains("action !in quickActions(provider)"))
        assertTrue(coordinator.contains("openOfficialFallback"))
        assertTrue(background.contains("ChatGptComposerOptionRequestCoordinator("))
        assertTrue(background.contains("composerOptionRequests.request(\"model\")"))
        assertTrue(optionInteraction.contains("pageAdapter()?.listModelOptions"))
        assertTrue(optionInteraction.contains("pageAdapter()?.listComposerTools"))
        assertTrue(optionRequests.contains("dismissMenu(requestId)"))
        assertTrue(optionRequests.contains("dispatchRequest(request.section, request.requestId)"))
        assertTrue(background.contains("adapter.startDictation"))
        val permissionController = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAudioPermissionController.kt",
        )
        val realtimeVoicePolicy = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptRealtimeVoicePolicy.kt",
        )
        assertTrue(realtimeVoicePolicy.contains("const val SEMANTIC = \"voice_mode\""))
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
        val background = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val handler = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTouchRequestHandler.kt",
        )

        assertTrue(dispatcher.contains("request.purpose in ALLOWED_PURPOSES"))
        assertTrue(dispatcher.contains("supportsEnhancedMode(webView.url)"))
        assertTrue(dispatcher.contains("webView.dispatchTouchEvent(down)"))
        assertTrue(dispatcher.contains("webView.dispatchTouchEvent(up)"))
        assertTrue(dispatcher.contains("open_model_submenu"))
        assertTrue(dispatcher.contains("open_composer_tools_submenu"))
        assertFalse(dispatcher.contains("Instrumentation"))
        assertFalse(dispatcher.contains("AccessibilityService"))
        assertTrue(handler.contains("adapter::collectModelOptions"))
        assertTrue(handler.contains("adapter::collectComposerTools"))
        assertTrue(background.contains("is ChatGptWebEvent.WebTouchRequest -> touchRequestHandler.handle(event)"))
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
