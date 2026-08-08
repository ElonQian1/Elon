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

        assertTrue(bridge.contains("WebViewCompat.addWebMessageListener"))
        assertTrue(bridge.contains("ALLOWED_ORIGIN = \"https://chatgpt.com\""))
        assertFalse(bridge.contains("addJavascriptInterface"))
        assertFalse(bridge.contains("getCookie("))
        assertTrue(adapter.contains("new MutationObserver"))
        assertTrue(adapter.contains("schema: 'yilong.ai.ui.v1'"))
        assertTrue(adapter.contains("authenticated: isAuthenticated()"))
        assertTrue(adapter.contains("url: location.origin + location.pathname"))
        assertFalse(adapter.contains("url: location.href"))
        assertTrue(adapter.contains("action === 'send_prompt'"))
        assertTrue(adapter.contains("action === 'start_google_login'"))
        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
            assertFalse("page adapter must not contain $it", adapter.contains(it))
        }
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
