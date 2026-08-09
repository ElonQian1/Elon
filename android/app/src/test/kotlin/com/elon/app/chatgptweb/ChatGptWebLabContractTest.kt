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
        val adapterLayout = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_layout.js"
        )

        assertTrue(bridge.contains("WebViewCompat.addWebMessageListener"))
        assertTrue(bridge.contains("ALLOWED_ORIGIN = \"https://chatgpt.com\""))
        assertTrue(
            bridge.indexOf("chatgpt_web_adapter_bootstrap.js") <
                bridge.indexOf("chatgpt_web_adapter_conversations.js")
        )
        assertFalse(bridge.contains("addJavascriptInterface"))
        assertFalse(bridge.contains("getCookie("))
        assertTrue(bootstrap.contains("window.__elonChatGptAdapterVersion = adapterVersion"))
        assertTrue(bootstrap.contains("previousBridge.dispose()"))
        assertTrue(bootstrap.contains("delete window[name]"))
        assertTrue(adapter.contains("new MutationObserver"))
        assertTrue(adapter.contains("adapterVersion,"))
        assertTrue(adapter.contains("observer.disconnect()"))
        assertTrue(adapter.contains("removeEventListener('popstate', scheduleSnapshot)"))
        assertTrue(adapter.contains("dispose"))
        assertTrue(adapter.contains("schema: 'yilong.ai.ui.v1'"))
        assertTrue(adapter.contains("authenticated: isAuthenticated()"))
        assertTrue(adapter.contains("url: location.origin + location.pathname"))
        assertTrue(adapter.contains("draft: composerValue(findComposer())"))
        assertTrue(adapter.contains("command.expectedDraft"))
        assertTrue(adapter.contains("document.execCommand('insertText'"))
        assertTrue(adapter.contains("Array.from(document.querySelectorAll(selector)).find(isVisible)"))
        assertTrue(adapter.contains("网页草稿已变化"))
        assertFalse(adapter.contains("url: location.href"))
        assertTrue(adapter.contains("action === 'send_prompt'"))
        assertTrue(adapter.contains("action === 'list_conversations'"))
        assertTrue(adapter.contains("action === 'open_conversation'"))
        assertTrue(adapter.contains("action === 'regenerate_response'"))
        assertTrue(adapter.contains("action === 'start_google_login'"))
        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
            assertFalse("page adapter must not contain $it", adapter.contains(it))
            assertFalse("conversation adapter must not contain $it", conversations.contains(it))
            assertFalse("message adapter must not contain $it", messages.contains(it))
        }
        assertTrue(conversations.contains("CONVERSATION_PATH"))
        assertTrue(conversations.contains("location.assign(new URL(path, location.origin).href)"))
        assertFalse(conversations.contains("location.href ="))
        assertTrue(messages.contains("function fencedCode"))
        assertTrue(messages.contains("function tableMarkdown"))
        assertTrue(messages.contains("function structuredParts"))
        assertTrue(messages.contains("lastStructuredTypes"))
        assertTrue(messages.contains("lastComplexOutput"))
        assertTrue(messages.contains("table, pre, blockquote, ol, ul"))
        assertTrue(messages.contains("complex_output"))
        assertTrue(messages.contains("message_regenerate"))
        assertTrue(adapterLayout.indexOf("read.aloud|朗读") < adapterLayout.indexOf("dictat|microphone|voice"))
        assertTrue(adapterLayout.contains("return 'sources'"))
        assertTrue(adapterLayout.contains("return 'more'"))
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
        assertTrue(modeController.contains("MotionEvent.ACTION_DOWN"))
        assertTrue(modeController.contains("WindowInsetsCompat.Type.ime()"))
        assertTrue(activity.contains("modeController.select(ChatGptWebModeController.Mode.NATIVE)"))
        assertTrue(activity.contains("loginController.onAuthenticated() || modeController.isQuickSelected()"))
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
        assertTrue(activity.contains("renderCapabilities(event.value.capabilities)"))
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
        assertFalse(adaptive.contains("emptyView.visibility"))
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
