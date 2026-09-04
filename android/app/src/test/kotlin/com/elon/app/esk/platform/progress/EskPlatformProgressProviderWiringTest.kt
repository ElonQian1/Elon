package com.elon.app.esk.platform.progress

import java.io.ByteArrayInputStream
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.*
import org.junit.Test
import org.w3c.dom.Element

/** Source/preview gates complement pure tests; no renderer or device result is implied. */
class EskPlatformProgressProviderWiringTest {
    private val activity get() = EskProgressProviderSources.kotlin("EskPlatformProgressConsentActivity.kt")

    @Test fun manifestAddsOnlyAnIndependentExplicitExportedProviderWithoutAliasOrFilter() {
        val manifest = xml(EskProgressProviderSources.read("android/app/src/main/AndroidManifest.xml"))
        val nodes = manifest.getElementsByTagName("activity")
        val providers = (0 until nodes.length).map { nodes.item(it) as Element }
            .filter { attr(it, "name") == ".esk.platform.progress.EskPlatformProgressConsentActivity" }
        assertEquals(1, providers.size)
        val provider = providers.single()
        assertEquals("true", attr(provider, "exported"))
        assertEquals("true", attr(provider, "excludeFromRecents"))
        assertEquals("false", attr(provider, "allowTaskReparenting"))
        assertEquals("standard", attr(provider, "launchMode"))
        assertEquals(0, provider.getElementsByTagName("intent-filter").length)
        val aliases = manifest.getElementsByTagName("activity-alias")
        for (i in 0 until aliases.length) assertFalse(attr(aliases.item(i) as Element, "targetActivity").contains("EskPlatformProgress"))
        val oldNames = listOf(".esk.platform.handoff.EskPlatformSnapshotConsentActivity", ".esk.handoff.EskSnapshotConsentActivity")
        oldNames.forEach { name -> assertEquals(1, (0 until nodes.length).count { attr(nodes.item(it) as Element, "name") == name }) }
    }

    @Test fun creationChecksOsCallerRequestAndHttpsBeforeReadingAnyAccountOrCredential() {
        ordered(method(activity, "onCreate"), "setResult(RESULT_CANCELED)", "FLAG_SECURE", "savedInstanceState != null",
            "hasOfficialEskPlatformProgressCaller()", "readEskPlatformProgressRequest(intent)",
            "EskPlatformProgressConsentView(this", "eskPlatformEndpoint(BuildConfig.SERVER_URL)",
            "EskPlatformSessionStore(this)", "source.capture()", "phase = Phase.CONFIRMING", "page.show(")
        val fields = activity.substringBefore("override fun onCreate")
        assertFalse(fields.contains("EskPlatformSessionStore(this)"))
        assertFalse(fields.contains("EskPlatformSellbackClient()"))
        assertTrue(method(activity, "onCreate").contains("request.getValue(\"cursor\").isNotEmpty()"))
    }

    @Test fun explicitConsentStartsOnlyOneBoundedGetPageAndCannotWriteOrAutomaticallyRestart() {
        val confirm = method(activity, "confirm")
        ordered(confirm, "phase != Phase.CONFIRMING", "liveAuthorization()", "val requestedCursor = cursor",
            "gate.begin(", "phase = Phase.READING", "page.loading()", "EskPlatformSellbackClient()", "Thread({",
            "source.page(BuildConfig.SERVER_URL, requestedCursor.takeIf { it.isNotEmpty() }) { captured.token }")
        assertEquals(1, Regex("source\\.page\\(").findAll(activity).count())
        ordered(confirm, "reader !== source || phase != Phase.READING", "!liveAuthorization()", "gate.consume(", "result.fold(")
        assertTrue(confirm.contains("SellbackNetworkFailure.CONFLICT"))
        assertTrue(confirm.contains("明确重新发起首页授权"))
        for (forbidden in listOf(".execute(", ".lookup(", ".lookupKey(", "SellbackAction", "source.page(BuildConfig.SERVER_URL, null",
            "postDelayed({ confirm", "returnSnapshot", "EskPlatformAccountReader", "EskSnapshotReader", "WebView"))
            assertFalse(forbidden, activity.contains(forbidden))
    }

    @Test fun callbackAndFinalReturnCheckCurrentIdentityBeforeFreshTimeAndSerialization() {
        ordered(method(activity, "liveAuthorization"), "!hasOfficialEskPlatformProgressCaller()", "sessions?.capture()",
            "captured.sameAs(current)", "System.currentTimeMillis()", "SystemClock.elapsedRealtime()")
        val output = method(activity, "returnProgress")
        ordered(output, "!liveAuthorization()", "val remaining", "captured.expiresAtMillis - System.currentTimeMillis()",
            "require(remaining > 0)", "Math.addExact(now, minOf(remaining, EskPlatformProgressContract.DISPLAY_WINDOW_MS))",
            "composeEskPlatformProgress(", "!liveAuthorization()", "eskPlatformProgressResult(", "SystemClock.elapsedRealtime()",
            "phase = Phase.FINISHED", "clearPrivateState()", "setResult(RESULT_OK, result)", "finish()")
        assertTrue(activity.contains("EskPlatformProgressContract.REQUEST_WINDOW_MS"))
        assertTrue(activity.contains("EskPlatformRequestGate.MAX_REQUEST_MS"))
    }

    @Test fun sessionChangeBackgroundRecreationAndObscuredTouchPurgeAndCancelLateCallbacks() {
        ordered(method(activity, "onCreate"), "EskPlatformSessionStore(this)", "revoked = true", "gate.invalidate()",
            "reader?.cancel()", "runOnUiThread { cancelAndFinish() }")
        ordered(method(activity, "clearPrivateState"), "revoked = true", "gate.invalidate()", "reader?.cancel()", "reader = null",
            "sessions?.close()", "sessions = null", "session = null", "nonce = null", "cursor = null",
            "handler.removeCallbacksAndMessages(null)", "page.clear()")
        for (name in listOf("onPause", "onStop", "onNewIntent", "onSaveInstanceState"))
            assertTrue(name, method(activity, name).contains("cancelAndFinish()"))
        assertTrue(method(activity, "onSaveInstanceState").contains("outState.clear()"))
        assertTrue(method(activity, "onDestroy").contains("clearPrivateState()"))
        ordered(method(activity, "dispatchTouchEvent"), "FLAG_WINDOW_IS_OBSCURED", "FLAG_WINDOW_IS_PARTIALLY_OBSCURED",
            "cancelAndFinish()", "return true", "super.dispatchTouchEvent(event)")
        ordered(method(activity, "fail"), "phase = Phase.FAILED", "clearPrivateState()", "page.unavailable(message)")
        assertFalse(activity.contains("savedInstanceState.get"))
        assertFalse(activity.contains("outState.put"))
    }

    @Test fun previewIsProductionScrollableTokenLayoutWithPrivateStateDisabled() {
        val source = EskProgressProviderSources.kotlin("EskPlatformProgressConsentView.kt")
        val layout = EskProgressProviderSources.read("android/app/src/main/res/layout/esk_platform_progress_consent_preview.xml")
        val document = xml(layout)
        assertEquals("ScrollView", document.documentElement.tagName)
        assertEquals("@color/elon_bg_app", attr(document.documentElement, "background"))
        assertEquals("true", attr(document.documentElement, "fillViewport"))
        val buttons = document.getElementsByTagName("Button")
        assertEquals(2, buttons.length)
        for (i in 0 until buttons.length) {
            val button = buttons.item(i) as Element
            assertEquals("52dp", attr(button, "minHeight"))
            assertEquals("wrap_content", attr(button, "layout_height"))
            assertEquals("match_parent", attr(button, "layout_width"))
        }
        for (marker in listOf("R.layout.esk_platform_progress_consent_preview", "activity.setContentView(root)",
            "IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS", "setOnApplyWindowInsetsListener", "insets.systemWindowInsetBottom",
            "button.filterTouchesWhenObscured = true", "view.isSaveEnabled = false", "view.isSaveFromParentEnabled = false",
            "view.setTextIsSelectable(false)", "Character.FORMAT", ".take(64)", "R.color.elon_titanium_mid"))
            assertTrue(marker, source.contains(marker))
        ordered(method(source, "clear"), "account.text =", "scope.text =", "status.text =", "primary.isEnabled = false", "primary.setOnClickListener(null)")
        assertTrue(source.contains("下一页仍需重新确认"))
        for (text in listOf("最多 20 条", "尚未上链", "未兑付", "可申请量不是可提现金额", "不提交、不取消申请",
            "不传登录凭据", "最多显示 60 秒", "不是实时余额", "无法远程撤回", "确认并读取本页进度"))
            assertTrue(text, layout.contains(text))
        assertFalse(Regex("#[0-9a-fA-F]{6,8}").containsMatchIn(layout + source))
    }

    @Test fun protocolNeverAcquiresCredentialPersistenceOrWebChannels() {
        val wire = EskProgressProviderSources.kotlin("EskPlatformProgressWire.kt")
        for (forbidden in listOf("getSharedPreferences", "FileOutputStream", "ClipboardManager", "WebView", "evaluateJavascript",
            "addJavascriptInterface", "Log.", "println(", "sendBroadcast", "startService", "putParcelable", "putSerializable"))
            assertFalse(forbidden, (activity + wire).contains(forbidden))
        val projection = wire.substringAfter("internal fun composeEskPlatformProgress").substringBefore("internal fun eskPlatformProgressResult")
        for (forbidden in listOf("record.key", "record.policy", "record.requestDigest", "summary.policy", "summary.enabled", "summary.reason"))
            assertFalse(forbidden, projection.contains(forbidden))
    }

    private fun ordered(source: String, vararg markers: String) {
        var previous = -1
        markers.forEach { marker ->
            val index = source.indexOf(marker, previous + 1)
            assertTrue("Missing or out-of-order: $marker", index > previous)
            previous = index
        }
    }
    private fun method(source: String, name: String): String {
        val signature = Regex("\\bfun\\s+${Regex.escape(name)}\\s*\\(").find(source) ?: error("Missing method $name")
        val start = source.indexOf('{', signature.range.last + 1)
        var depth = 0
        for (index in start until source.length) {
            if (source[index] == '{') depth++
            if (source[index] == '}' && --depth == 0) return source.substring(start, index + 1)
        }
        error("Unclosed method $name")
    }
    private fun xml(source: String) = DocumentBuilderFactory.newInstance().apply {
        isNamespaceAware = true
        setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
    }.newDocumentBuilder().parse(ByteArrayInputStream(source.toByteArray(Charsets.UTF_8)))
    private fun attr(element: Element, name: String) = element.getAttributeNS("http://schemas.android.com/apk/res/android", name)
}
