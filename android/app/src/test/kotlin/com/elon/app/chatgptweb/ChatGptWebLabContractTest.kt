package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebLabContractTest {
    @Test
    fun activityPersistsWebViewCookiesWithoutExportingThem() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )

        assertTrue(activity.contains("setAcceptCookie(true)"))
        assertTrue(activity.contains("setAcceptThirdPartyCookies(binding.chatGptWebView, true)"))
        assertTrue(activity.contains("cookieManager.flush()"))
        assertTrue(activity.contains("removeAllCookies"))
        assertFalse(activity.contains("getCookie("))
        assertFalse(activity.contains("addJavascriptInterface"))
        assertFalse(activity.contains("evaluateJavascript"))
        assertFalse(activity.contains("OkHttpClient"))
    }

    @Test
    fun enhancedBridgeIsOriginScopedAndDoesNotReplayProviderTraffic() {
        val bridge = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt"
        )
        val adapter = readRepositoryFile("android/app/src/main/assets/chatgpt_web_adapter.js")
        val bootstrap = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_bootstrap.js"
        )
        val conversations = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js"
        )
        val messages = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_messages.js"
        )
        val composer = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_composer.js"
        )
        val adapterLayout = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_layout.js"
        )
        val pageSemanticPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_page_semantic_policy.js"
        )
        val navigationPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_navigation_policy.js"
        )
        val formControls = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_form_controls.js"
        )
        val controlOwnershipPolicy = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_control_ownership_policy.js"
        )
        val formCommands = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_form_commands.js"
        )
        val disclosureControls = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_disclosure_controls.js"
        )

        assertTrue(bridge.contains("WebViewCompat.addWebMessageListener"))
        assertTrue(bridge.contains("ALLOWED_ORIGIN = \"https://chatgpt.com\""))
        assertTrue(
            bridge.indexOf("chatgpt_web_adapter_bootstrap.js") <
                bridge.indexOf("chatgpt_web_adapter_conversations.js")
        )
        assertTrue(
            bridge.indexOf("chatgpt_web_adapter_navigation_policy.js") <
                bridge.indexOf("chatgpt_web_adapter_navigation.js") &&
                bridge.indexOf("chatgpt_web_adapter_navigation.js") <
                bridge.indexOf("chatgpt_web_adapter_page_semantic_policy.js")
        )
        assertTrue(
                bridge.indexOf("chatgpt_web_adapter_page_semantic_policy.js") <
                bridge.indexOf("chatgpt_web_adapter_form_controls.js") &&
                bridge.indexOf("chatgpt_web_adapter_form_controls.js") <
                bridge.indexOf("chatgpt_web_adapter_control_ownership_policy.js") &&
                bridge.indexOf("chatgpt_web_adapter_control_ownership_policy.js") <
                bridge.indexOf("chatgpt_web_adapter_form_commands.js") &&
                bridge.indexOf("chatgpt_web_adapter_form_commands.js") <
                bridge.indexOf("chatgpt_web_adapter_disclosure_controls.js") &&
                bridge.indexOf("chatgpt_web_adapter_disclosure_controls.js") <
                bridge.indexOf("chatgpt_web_adapter_layout.js")
        )
        assertFalse(bridge.contains("addJavascriptInterface"))
        assertFalse(bridge.contains("getCookie("))
        assertTrue(
            bootstrap.contains("Number(window.__elonChatGptAdapterTargetVersion || 0)"),
        )
        assertTrue(bridge.contains("window.__elonChatGptAdapterTargetVersion=\$ADAPTER_VERSION;"))
        assertTrue(bootstrap.contains("window.__elonChatGptAdapterVersion = adapterVersion"))
        assertTrue(bootstrap.contains("previousBridge.dispose()"))
        assertTrue(bootstrap.contains("delete window[name]"))
        assertTrue(adapter.contains("new MutationObserver"))
        assertTrue(adapter.contains("adapterVersion,"))
        assertTrue(adapter.contains("observedMessageCount:"))
        assertTrue(adapter.contains("messageWindowStart:"))
        assertTrue(messages.contains("function readMessageWindow(streaming)"))
        assertTrue(navigationPolicy.contains("function isProjectRoute(path)"))
        assertTrue(navigationPolicy.contains("function isConversationPath(path)"))
        assertTrue(adapter.contains("observer.disconnect()"))
        assertTrue(adapter.contains("removeEventListener('popstate', scheduleSnapshot)"))
        assertTrue(adapter.contains("dispose"))
        assertTrue(adapter.contains("schema: 'yilong.ai.ui.v1'"))
        assertTrue(adapter.contains("authenticated: isAuthenticated(dictationActive, loginRequired)"))
        assertTrue(adapter.contains("pageKind,"))
        assertTrue(adapter.contains("loginRequired,"))
        assertTrue(adapterLayout.contains("window.__elonChatGptLayout = Object.freeze({"))
        assertTrue(adapterLayout.contains("requestSemanticTouch"))
        assertTrue(adapter.contains("url: location.origin + location.pathname"))
        assertTrue(adapter.contains("draft: composerValue(composer)"))
        assertTrue(adapter.contains("capabilities: detectCapabilities(findComposer())"))
        assertTrue(adapter.contains("command.expectedDraft"))
        assertTrue(adapter.contains("command.requestId"))
        assertTrue(adapter.contains("event.requestId = requestId"))
        assertTrue(bridge.contains("put(\"requestId\", requestId)"))
        assertTrue(adapter.contains("document.execCommand('insertText'"))
        assertTrue(adapter.contains("Array.from(document.querySelectorAll(selector)).find(isVisible)"))
        assertTrue(adapter.contains("网页草稿已变化"))
        assertFalse(adapter.contains("url: location.href"))
        assertTrue(adapter.contains("action === 'send_prompt'"))
        assertTrue(adapter.contains("action === 'list_conversations'"))
        assertTrue(adapter.contains("action === 'open_conversation'"))
        assertTrue(adapter.contains("action === 'set_ui_control_text'"))
        assertTrue(adapter.contains("action === 'set_ui_control_slider'"))
        assertTrue(adapter.contains("action === 'set_ui_control_expanded'"))
        assertTrue(composer.contains("function ownsOptionNode"))
        assertTrue(adapterLayout.contains("composerAdapter.ownsOptionNode(node)"))
        assertTrue(conversations.contains("if (location.pathname === '/')"))
        assertTrue(adapter.contains("action === 'regenerate_response'"))
        assertTrue(adapter.contains("action === 'start_google_login'"))
        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
            assertFalse("page adapter must not contain $it", adapter.contains(it))
            assertFalse("conversation adapter must not contain $it", conversations.contains(it))
            assertFalse("message adapter must not contain $it", messages.contains(it))
            assertFalse("form adapter must not contain $it", formControls.contains(it))
            assertFalse("form command adapter must not contain $it", formCommands.contains(it))
            assertFalse("disclosure adapter must not contain $it", disclosureControls.contains(it))
        }
        assertTrue(disclosureControls.contains("function setExpanded"))
        assertTrue(disclosureControls.contains("purpose: 'invoke_ui_control'"))
        assertTrue(formControls.contains("ACTIONABLE_SELECTOR"))
        assertTrue(formControls.contains("function setText"))
        assertTrue(formControls.contains("function planSelectedState"))
        assertTrue(formControls.contains("function selectChoice"))
        assertTrue(formControls.contains("function setSliderValue"))
        assertTrue(formCommands.contains("purpose: 'invoke_ui_control'"))
        assertTrue(formControls.contains("kind === 'password'"))
        assertFalse(formControls.contains("node.value ||"))
        assertTrue(controlOwnershipPolicy.contains("isPrimaryComposerTextControl"))
        assertTrue(adapterLayout.contains("controlOwnershipPolicy.isPrimaryComposerTextControl"))
        assertTrue(conversations.contains("CONVERSATION_PATH"))
        assertTrue(conversations.contains("location.assign(new URL(path, location.origin).href)"))
        assertFalse(conversations.contains("location.href ="))
        assertTrue(messages.contains("function fencedCode"))
        assertTrue(messages.contains("function tableMarkdown"))
        assertTrue(messages.contains("function structuredParts"))
        assertTrue(messages.contains("FILE_PATH_EXTENSION"))
        assertTrue(messages.contains("COMPLEX_PART_TYPES"))
        assertTrue(messages.contains("data-testid*=\"math\""))
        assertTrue(messages.contains("data-testid*=\"chart\""))
        assertTrue(messages.contains("data-testid*=\"map\""))
        assertTrue(messages.contains("data-testid*=\"interactive\""))
        assertTrue(messages.contains("tag === 'DETAILS'"))
        assertTrue(messages.contains("tag === 'DL'"))
        assertFalse(messages.contains("/\\.[a-z0-9]{2,8}$/i.test(path)"))
        assertTrue(messages.contains("lastStructuredTypes"))
        assertTrue(messages.contains("lastComplexOutput"))
        assertTrue(messages.contains("table, pre, blockquote, ol, ul"))
        assertTrue(messages.contains("complex_output"))
        assertTrue(messages.contains("message_regenerate"))
        assertTrue(adapterLayout.indexOf("read.aloud|朗读") < adapterLayout.indexOf("dictat|听写|语音输入"))
        assertTrue(adapterLayout.contains("return 'sources'"))
        assertTrue(adapterLayout.contains("return 'create_asset'"))
        assertTrue(adapterLayout.contains("return 'voice_mode'"))
        assertTrue(adapterLayout.contains("return 'open_media'"))
        assertTrue(adapterLayout.contains("return 'reasoning_details'"))
        assertTrue(adapterLayout.contains("文件和来源|查看来源|来源"))
        assertTrue(adapterLayout.contains("return 'conversation_files'"))
        assertTrue(adapterLayout.contains("return 'pin'"))
        assertTrue(adapterLayout.contains("return 'archive'"))
        assertTrue(adapterLayout.contains("在聊天中查看文件"))
        assertTrue(adapterLayout.contains("取消置顶|置顶聊天"))
        assertTrue(adapterLayout.contains("取消归档|归档"))
        assertTrue(adapterLayout.contains("return 'more'"))
        assertTrue(adapterLayout.contains("function addPageContentControls"))
        assertTrue(adapterLayout.contains("pageKind() !== 'feature'"))
        assertTrue(adapterLayout.contains("addRegionControls(target, content, 'content'"))
        assertTrue(adapterLayout.contains("function compatibilityFor(controls, kind)"))
        assertTrue(adapterLayout.contains("hasFeatureContent"))
        assertTrue(adapterLayout.contains("compatibility: compatibilityFor(controls, kind)"))
        assertTrue(adapterLayout.contains("window.__elonChatGptPageSemanticPolicy.classify"))
        assertTrue(pageSemanticPolicy.contains("/^\\/scheduled(?:\\/|$)/"))
        assertTrue(pageSemanticPolicy.contains("/^\\/library(?:\\/|$)/"))
        assertTrue(pageSemanticPolicy.contains("/^\\/plugins(?:\\/|$)/"))
        assertTrue(pageSemanticPolicy.contains("\\/project(?:\\/|$)/"))
        assertTrue(ChatGptWebUiSemantics.KNOWN.contains("apps"))
        assertTrue(pageSemanticPolicy.contains("semantic: 'tasks'"))
        assertTrue(pageSemanticPolicy.contains("semantic: 'library'"))
        assertTrue(pageSemanticPolicy.contains("semantic: 'apps'"))
        assertTrue(pageSemanticPolicy.contains("semantic: 'project'"))
        assertTrue(pageSemanticPolicy.contains("return 'navigation'"))
        assertTrue(adapterLayout.contains("const MAX_DISCOVERED_CONTROLS = 512"))
        assertTrue(adapterLayout.contains("discoveredControlCount: discovery.totalCount"))
        assertTrue(adapterLayout.contains("controlsTruncated: discovery.truncated"))
        assertFalse(adapterLayout.contains("return controls.slice(0, 160)"))
        assertTrue(adapterLayout.contains("turns.some((turn) => turn.contains(node))"))
        assertTrue(adapterLayout.contains("node.closest('aside, nav, [role=\"navigation\"]')"))
        assertTrue(adapterLayout.contains("[header, composer, suggestions].concat(overlays)"))
        assertTrue(adapterLayout.contains("const overlay = overlays[overlays.length - 1]"))
        listOf("personalization", "help", "logout", "plan").forEach { semantic ->
            assertTrue(ChatGptWebUiSemantics.KNOWN.contains(semantic))
            assertTrue(pageSemanticPolicy.contains("return '$semantic'"))
        }
        val semanticFunction = adapterLayout.substring(
            adapterLayout.indexOf("function semanticFor"),
            adapterLayout.indexOf("function defaultLabel"),
        )
        val emittedSemantics = Regex("return '([a-z_]+)'")
            .findAll(semanticFunction)
            .map { it.groupValues[1] }
            .toSet()
        assertTrue(ChatGptWebUiSemantics.KNOWN.containsAll(emittedSemantics))
    }

    @Test
    fun nativeModeKeepsTheOfficialWebViewActiveBehindTheOpaqueShell() {
        val modeController = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebModeController.kt"
        )
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        assertTrue(modeController.contains("webView.visibility = View.VISIBLE"))
        assertFalse(modeController.contains("webView.visibility = View.INVISIBLE"))
        assertTrue(modeController.contains("chromeViews.forEach { it.visibility = chromeVisibility }"))
        assertTrue(modeController.contains("if (mode == Mode.WEB) View.GONE else View.VISIBLE"))
        assertTrue(modeController.contains("fun exitOfficialView(): Boolean"))
        assertTrue(modeController.contains("MotionEvent.ACTION_DOWN"))
        assertTrue(modeController.contains("WindowInsetsCompat.Type.ime()"))
        assertTrue(activity.contains("binding.chatGptWebToolbar"))
        assertTrue(activity.contains("ChatGptWebBackNavigation.Action.EXIT_OFFICIAL_VIEW"))
        assertTrue(activity.contains("modeController.select(ChatGptWebModeController.Mode.NATIVE)"))
        assertTrue(activity.contains("sessionRestorer.restorePreferredMode("))
    }

    @Test
    fun mcpCommandsCarryTheirReceiptIdToTheOfficialPage() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val actions = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpActions.kt"
        )
        val commandPort = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebMcpCommandPort.kt"
        )
        val conversation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeConversationController.kt"
        )

        assertTrue(actions.contains("private val commands: ChatGptWebMcpCommandPort"))
        assertTrue(actions.contains("block(request.id)"))
        assertTrue(activity.contains("commands = mcpCommandPort"))
        assertTrue(activity.contains("nativeController.submitFromMcp(requestId)"))
        assertTrue(conversation.contains("fun submitFromMcp(requestId: String)"))
        assertTrue(commandPort.contains("fun sendInput(requestId: String)"))
        assertTrue(commandPort.contains("fun invokeControl(controlId: String, requestId: String)"))
        assertTrue(commandPort.contains("fun setControlText(controlId: String, text: String, requestId: String)"))
        assertTrue(commandPort.contains("fun setControlSelected(controlId: String, selected: Boolean, requestId: String)"))
        assertTrue(commandPort.contains("fun selectControlChoice(controlId: String, choiceIndex: Int, requestId: String)"))
        assertTrue(commandPort.contains("fun setControlSlider(controlId: String, value: Double, requestId: String)"))
        assertTrue(commandPort.contains("fun setControlExpanded(controlId: String, expanded: Boolean, requestId: String)"))
    }

    @Test
    fun nativeFeatureFormsUseStableSelectorsWithoutPrefillingPrivateValues() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val overlay = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeOverlayControlsController.kt"
        )
        val dialog = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeFormControlDialog.kt"
        )

        assertTrue(activity.contains("onSetText = { controlId, text ->"))
        assertTrue(activity.contains("pageAdapter.setUiControlText(controlId, text)"))
        assertTrue(overlay.contains("if (control.supportsTextEntry)"))
        assertTrue(overlay.contains("ChatGptNativeFormControlDialog.show"))
        assertTrue(dialog.contains("chatgpt-control-input:"))
        assertTrue(dialog.contains("chatgpt-control-input-commit:"))
        assertTrue(dialog.contains("require(control.supportsTextEntry)"))
        assertFalse(dialog.contains("setText(control."))
    }

    @Test
    fun nativeFeatureChoicesExposeStableIndexedSelectorsWithoutOptionValues() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val overlay = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeOverlayControlsController.kt"
        )
        val dialog = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeChoiceControlDialog.kt"
        )

        assertTrue(activity.contains("pageAdapter.selectUiControlChoice(controlId, choiceIndex)"))
        assertTrue(overlay.contains("control.supportsChoiceSelection"))
        assertTrue(overlay.contains("ChatGptNativeChoiceControlDialog.show"))
        assertTrue(dialog.contains("chatgpt-control-choice:"))
        assertTrue(dialog.contains("control.choiceLabels[position]"))
        assertFalse(dialog.contains("option.value"))
    }

    @Test
    fun nativeSlidersExposeStableSelectorsAndBoundedValues() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val overlay = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeOverlayControlsController.kt"
        )
        val dialog = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeSliderControlDialog.kt"
        )

        assertTrue(activity.contains("pageAdapter.setUiControlSlider(controlId, value)"))
        assertTrue(overlay.contains("control.supportsSliderValue"))
        assertTrue(overlay.contains("ChatGptNativeSliderControlDialog.show"))
        assertTrue(dialog.contains("chatgpt-control-slider:"))
        assertTrue(dialog.contains("chatgpt-control-slider-value:"))
        assertTrue(dialog.contains("chatgpt-control-slider-commit:"))
        assertTrue(dialog.contains("max = slider.stepCount"))
    }

    @Test
    fun bridgeInstallationWaitsUntilEveryStateConsumerIsInitialized() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val install = activity.indexOf("pageAdapter.install()")

        assertTrue(install > activity.indexOf("modeController = ChatGptWebModeController("))
        assertTrue(install > activity.indexOf("loginController = ChatGptNativeLoginController("))
        assertTrue(install > activity.indexOf("googleAccountHintController = ChatGptGoogleAccountHintController("))
        assertTrue(install > activity.indexOf("conversationListController = ChatGptNativeConversationListController("))
        assertTrue(install > activity.indexOf("modeController.attach()"))
    }

    @Test
    fun nativeConversationHistoryIsCapabilityGatedAndUsesThePageAdapter() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeConversationListController.kt"
        )
        val layout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_chatgpt_web_test.xml"
        )

        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeHistory\""))
        assertTrue(activity.contains("pageAdapter.listConversations()"))
        assertTrue(activity.contains("pageAdapter.openConversation(path)"))
        assertTrue(activity.contains("conversationListController.render(event.conversations)"))
        assertTrue(activity.contains("renderCapabilities(snapshot.capabilities)"))
        assertTrue(controller.contains("ChatGptWebCapabilityId.CONVERSATION_LIST"))
        assertTrue(controller.contains("bridgeReady && listSupported"))
    }

    @Test
    fun nativeMessagesUseDedicatedMarkdownRenderingAndCapabilityGatedActions() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeConversationController.kt"
        )
        val adapter = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeMessageAdapter.kt"
        )
        val partRenderer = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeMessagePartRenderer.kt"
        )
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )

        assertTrue(controller.contains("ChatGptNativeMessageAdapter("))
        assertFalse(controller.contains("ChatAdapter("))
        assertTrue(adapter.contains("markwon.setMarkdown"))
        assertTrue(adapter.contains("ChatGptWebCapabilityId.MESSAGE_REGENERATE"))
        assertTrue(adapter.contains("partRenderer.render"))
        assertTrue(partRenderer.contains("ChatGptWebMessagePart"))
        assertTrue(partRenderer.contains("messagePartSelector"))
        assertTrue(partRenderer.contains("onOpenOfficial"))
        assertFalse(partRenderer.contains("OkHttpClient"))
        assertFalse(partRenderer.contains("getCookie("))
        assertTrue(adapter.contains("position == messages.indexOfLast"))
        assertTrue(activity.contains("pageAdapter.regenerateResponse()"))
    }

    @Test
    fun conversationControllerExclusivelyOwnsTheSharedEmptyState() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val conversation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeConversationController.kt"
        )
        val adaptive = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeAdaptiveUiController.kt"
        )

        assertTrue(activity.contains("onSuggestionsVisibleChanged = nativeController::setSuggestionsVisible"))
        assertTrue(conversation.contains("messages.isEmpty() && !suggestionsVisible"))
        assertTrue(adaptive.contains("ChatGptNativeControlPresentation.directSelector(control)"))
        assertFalse(adaptive.contains("emptyView.visibility"))
    }

    @Test
    fun nativeOverlayKeepsOfficialActionsDiscoverableWithoutInterruptingOtherSurfaces() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val overlay = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeOverlayControlsController.kt"
        )
        val controlDialog = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeControlDialog.kt"
        )

        assertTrue(overlay.contains("it.semantic == \"timestamp\""))
        assertTrue(overlay.contains("ChatGptNativeControlPresentation.pageActions(value.controls)"))
        assertTrue(overlay.contains("ChatGptNativeControlPresentation.pageActionsSelector(controls)"))
        assertTrue(overlay.contains("setOnClickListener { showActions() }"))
        assertTrue(overlay.contains("controlsChanged && dialog?.isShowing == true"))
        assertFalse(overlay.contains("controlsChanged && shouldPresent()"))
        assertTrue(
            overlay.contains(
                "activity.getString(R.string.chatgpt_official_page_actions) + \" · \" + label"
            )
        )
        assertFalse(overlay.contains("setMessage"))
        assertTrue(controlDialog.contains("override fun isEnabled(position: Int)"))
        assertTrue(controlDialog.contains("textView.isEnabled = control.enabled"))
        assertFalse(activity.contains("shouldPresent ="))
    }

    @Test
    fun androidBackDismissesTheOfficialOverlayBeforeLeavingChatGpt() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )

        assertTrue(activity.contains("ChatGptWebBackNavigation.decide"))
        assertTrue(activity.contains("KeyEvent.KEYCODE_ESCAPE"))
        assertTrue(activity.contains("pageAdapter::requestUiManifest"))
    }

    @Test
    fun quickLoginUsesOurShellAndKeepsCredentialsOnOfficialPages() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeLoginController.kt"
        )
        val googleController = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptGoogleAccountHintController.kt"
        )
        val webViewClient = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebViewClient.kt"
        )
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")
        val layout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_chatgpt_web_test.xml"
        )
        val loginRootStart = layout.indexOf("android:id=\"@+id/chatGptQuickRoot\"")
        val loginRootEnd = layout.indexOf("android:id=\"@+id/chatGptNativeRoot\"")
        val loginShell = layout.substring(loginRootStart, loginRootEnd)

        assertTrue(layout.contains("android:id=\"@+id/chatGptModeQuick\""))
        assertTrue(loginShell.contains("android:id=\"@+id/chatGptQuickGoogle\""))
        assertTrue(loginShell.contains("android:id=\"@+id/chatGptQuickLogin\""))
        assertTrue(loginShell.contains("android:id=\"@+id/chatGptQuickOfficial\""))
        assertFalse(loginShell.contains("<EditText"))
        assertTrue(activity.contains("loadUrl(ChatGptWebNavigationPolicy.AUTH_URL)"))
        val restorer = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSessionRestorer.kt"
        )
        assertTrue(activity.contains("ChatGptWebSessionRestorer(this)"))
        assertTrue(activity.contains("sessionRestorer.restoreUrl()"))
        assertTrue(activity.contains("sessionRestorer.onPageReady(url)"))
        assertTrue(activity.contains("sessionRestorer.onModeShown(mode)"))
        assertTrue(activity.contains("sessionRestorer.restorePreferredMode("))
        assertTrue(restorer.contains("ChatGptWebSessionStateStore(context)"))
        assertTrue(restorer.contains("pendingMode = stateStore.restoreMode()"))
        assertTrue(restorer.contains("stateStore.saveMode(mode)"))
        assertTrue(activity.contains("loginController.onAuthenticated()"))
        assertTrue(googleController.contains("AccountManager.newChooseAccountIntent"))
        assertTrue(googleController.contains("GOOGLE_ACCOUNT_TYPE = \"com.google\""))
        assertTrue(webViewClient.contains("request.method.equals(\"GET\""))
        assertFalse(manifest.contains("android.permission.GET_ACCOUNTS"))
        listOf("getCookie(", "Authorization", "OkHttpClient", "evaluateJavascript").forEach {
            assertFalse("quick login controller must not contain $it", controller.contains(it))
        }
        listOf(
            "getAuthToken",
            "idToken",
            "SharedPreferences",
            "getCookie(",
            "Authorization:",
        ).forEach {
            assertFalse("Google account hint controller must not contain $it", googleController.contains(it))
        }
    }

    @Test
    fun webViewEnablesSystemWebAuthenticationWithoutHandlingCredentials() {
        val support = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAuthenticationSupport.kt"
        )
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val build = readRepositoryFile("android/app/build.gradle")

        assertTrue(build.contains("androidx.webkit:webkit:1.12.1"))
        assertTrue(support.contains("WebViewFeature.WEB_AUTHENTICATION"))
        assertTrue(support.contains("WebSettingsCompat.setWebAuthenticationSupport"))
        assertTrue(support.contains("WEB_AUTHENTICATION_SUPPORT_FOR_BROWSER"))
        assertTrue(activity.contains("ChatGptWebAuthenticationSupport.configure(settings)"))
        listOf(
            "AccountManager",
            "getAuthToken",
            "GoogleIdToken",
            "document.cookie",
            "getCookie(",
            "Authorization:",
        ).forEach {
            assertFalse("WebAuthn support must not contain $it", support.contains(it))
        }
    }

    @Test
    fun initialPageLoadWaitsForLocalProxyPreparationWithoutExportingCredentials() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )
        val proxy = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebProxyController.kt"
        )

        assertTrue(activity.contains("proxyController.prepare { status ->"))
        assertTrue(proxy.contains("connectivityManager.defaultProxy"))
        assertTrue(proxy.contains("NetworkCapabilities.TRANSPORT_VPN"))
        assertTrue(proxy.contains("WebViewFeature.PROXY_OVERRIDE"))
        assertTrue(proxy.contains("ProxyController.getInstance().setProxyOverride"))
        assertTrue(proxy.contains("clearProxyOverride"))
        assertFalse(proxy.contains("getCookie("))
        assertFalse(proxy.contains("Authorization"))
    }

    @Test
    fun webViewAccountIsOwnerAppOnlyAndReachableFromProfileAndProviderSettings() {
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")
        val providerActivity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/AiProviderAccountsActivity.kt"
        )
        val providerLayout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_ai_provider_accounts.xml"
        )
        val profileEntry = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProfileChatGptWebEntry.kt"
        )
        val profileActions = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainProfileQuickActions.kt"
        )

        val declarationStart = manifest.indexOf("android:name=\".chatgptweb.ChatGptWebTestActivity\"")
        assertTrue(declarationStart >= 0)
        val declarationEnd = manifest.indexOf("/>", declarationStart)
        val declaration = manifest.substring(declarationStart, declarationEnd)
        assertTrue(declaration.contains("android:exported=\"false\""))
        assertTrue(providerActivity.contains("ChatGptWebTestActivity::class.java"))
        assertTrue(providerLayout.contains("android:id=\"@+id/aiProviderChatGptWebLab\""))
        assertTrue(profileEntry.contains("ChatGPT 网页账号"))
        assertTrue(profileEntry.contains("ChatGptWebTestActivity::class.java"))
        assertTrue(profileActions.contains("chatGptWebEntry.attach()"))
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
