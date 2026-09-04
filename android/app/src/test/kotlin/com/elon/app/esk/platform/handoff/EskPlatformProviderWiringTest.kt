package com.elon.app.esk.platform.handoff

import com.elon.eskcontract.EskSnapshotContract
import java.io.ByteArrayInputStream
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.w3c.dom.Element

/** Source integration checks; not evidence of Android/device lifecycle or real-account acceptance. */
class EskPlatformProviderWiringTest {
    @Test
    fun providerHasItsOwnExplicitExportedComponentWithoutAliasesOrIntentFilters() {
        val manifest = xml(read("android/app/src/main/AndroidManifest.xml"))
        val nodes = manifest.getElementsByTagName("activity")
        val activities = (0 until nodes.length).map { nodes.item(it) as Element }
        val provider = activities.filter { android(it, "name") == PROVIDER }
        assertEquals(1, provider.size)
        assertEquals("true", android(provider.single(), "exported"))
        assertEquals("true", android(provider.single(), "excludeFromRecents"))
        assertEquals("false", android(provider.single(), "allowTaskReparenting"))
        assertEquals("standard", android(provider.single(), "launchMode"))
        assertEquals(0, provider.single().getElementsByTagName("intent-filter").length)
        val aliases = manifest.getElementsByTagName("activity-alias")
        (0 until aliases.length).forEach {
            assertFalse(android(aliases.item(it) as Element, "targetActivity")
                .endsWith("EskPlatformSnapshotConsentActivity"))
        }
        assertEquals("false", android(activities.single {
            android(it, "name") == ".esk.platform.EskPlatformAssetsActivity"
        }, "exported"))
        assertEquals("true", android(activities.single {
            android(it, "name") == ".esk.handoff.EskSnapshotConsentActivity"
        }, "exported"))
    }

    @Test
    fun creationValidatesCallerRequestAndHttpsBeforeAnySessionRead() {
        val activity = source("EskPlatformSnapshotConsentActivity.kt")
        val create = method(activity, "onCreate")
        ordered(create, "setResult(RESULT_CANCELED)", "FLAG_SECURE",
            "savedInstanceState != null || !hasOfficialEskPlatformSnapshotCaller()",
            "readEskPlatformSnapshotRequest(intent)", "EskPlatformSnapshotConsentView(this, ::cancelAndFinish)",
            "eskPlatformEndpoint(BuildConfig.SERVER_URL) == null", "return fail(",
            "EskPlatformSessionStore(this)", "source.capture()", "phase = Phase.CONFIRMING",
            "page.show(captured.displayName, ::confirm)")
        val guard = create.substring(create.indexOf("if (eskPlatformEndpoint"), create.indexOf("val source ="))
        assertTrue(guard.contains("return fail("))
        listOf(".fetch(", "EskPlatformAccountReader()", "confirm()").forEach {
            assertFalse("No reading or auto-consent during creation: $it", create.contains(it))
        }
        assertTrue(create.contains("EskPlatformSnapshotContract.REQUEST_WINDOW_MS"))
        val fields = activity.substring(0, activity.indexOf("override fun onCreate"))
        assertFalse(fields.contains("EskPlatformSessionStore(this)"))
        assertFalse(fields.contains("AuthManager."))
        assertFalse(fields.contains(".capture("))
    }

    @Test
    fun onlyExplicitFreshConsentStartsTheSingleFormalRead() {
        val activity = source("EskPlatformSnapshotConsentActivity.kt")
        val confirm = method(activity, "confirm")
        ordered(confirm, "if (phase != Phase.CONFIRMING) return", "if (!liveAuthorization())",
            "gate.begin(captured,", "phase = Phase.READING", "page.loading()",
            "EskPlatformAccountReader()", "source.fetch(BuildConfig.SERVER_URL) { captured.token }")
        assertEquals(1, Regex("\\.fetch\\(").findAll(activity).count())
        assertEquals(1, Regex("::confirm\\b").findAll(activity).count())
        listOf("onResume", "onNewIntent", "onCreate").forEach {
            assertFalse(method(activity, it).contains(".fetch("))
            assertFalse(method(activity, it).contains("confirm()"))
        }
        ordered(confirm, "if (reader !== source || phase != Phase.READING)",
            "if (!liveAuthorization() || !gate.consume(ticket, sessions?.capture()",
            "return@runOnUiThread cancelAndFinish()", "result.fold(onSuccess = ::returnSnapshot")
        assertTrue(confirm.contains("EskPlatformRequestGate.MAX_REQUEST_MS"))
        assertTrue(confirm.contains("phase == Phase.READING && reader === source"))
        val live = method(activity, "liveAuthorization")
        listOf("!revoked", "foreground", "!isFinishing", "!isDestroyed",
            "validWindow(startedAt, SystemClock.elapsedRealtime())", "captured.validAt(System.currentTimeMillis())",
            "captured.sameAs(sessions?.capture())", "hasOfficialEskPlatformSnapshotCaller()",
        ).forEach { assertTrue("Authorization missing $it", live.contains(it)) }
    }

    @Test
    fun resultIsOneShotRecheckedAndLimitedByMonotonicTimeAndSessionExpiry() {
        val activity = source("EskPlatformSnapshotConsentActivity.kt")
        val result = method(activity, "returnSnapshot")
        ordered(result, "phase != Phase.READING || !liveAuthorization()", "nonce ?: return cancelAndFinish()",
            "SystemClock.elapsedRealtime()", "captured.expiresAtMillis - epoch", "require(remaining > 0)",
            "Math.addExact(now, minOf(remaining, EskPlatformSnapshotContract.DISPLAY_WINDOW_MS))",
            "composeEskPlatformSnapshot(account, expectedNonce, startedAt, now, expires)",
            "eskPlatformSnapshotResult(fields, expectedNonce, startedAt, SystemClock.elapsedRealtime())",
            "if (!liveAuthorization()) return cancelAndFinish()", "phase = Phase.FINISHED",
            "clearPrivateState()", "setResult(RESULT_OK, result)", "finish()")
        assertEquals(1, Regex("setResult\\(RESULT_OK").findAll(activity).count())
        assertFalse(result.contains("epoch, expires"))
        val cancel = method(activity, "cancelAndFinish")
        ordered(cancel, "if (phase == Phase.FINISHED) return", "phase = Phase.FINISHED",
            "clearPrivateState()", "setResult(RESULT_CANCELED)", "finish()")
        assertFalse(method(activity, "fail").contains("RESULT_OK"))
    }

    @Test
    fun lifecycleAndAuthInvalidationClearConsentAndRejectLaterCallbacks() {
        val activity = source("EskPlatformSnapshotConsentActivity.kt")
        ordered(method(activity, "onCreate"), "EskPlatformSessionStore(this)", "revoked = true",
            "gate.invalidate()", "reader?.cancel()", "runOnUiThread { cancelAndFinish() }")
        ordered(method(activity, "clearPrivateState"), "revoked = true", "gate.invalidate()", "reader?.cancel()",
            "reader = null", "sessions?.close()", "sessions = null", "session = null", "nonce = null",
            "handler.removeCallbacksAndMessages(null)", "page.clear()")
        ordered(method(activity, "fail"), "phase = Phase.FAILED", "clearPrivateState()", "page.unavailable(message)")
        ordered(method(activity, "onPause"), "foreground = false", "cancelAndFinish()", "super.onPause()")
        ordered(method(activity, "onStop"), "cancelAndFinish()", "super.onStop()")
        assertTrue(method(activity, "onNewIntent").contains("cancelAndFinish()"))
        ordered(method(activity, "onSaveInstanceState"), "cancelAndFinish()", "super.onSaveInstanceState(outState)", "outState.clear()")
        ordered(method(activity, "onDestroy"), "foreground = false", "phase = Phase.FINISHED",
            "clearPrivateState()", "super.onDestroy()")
        ordered(method(activity, "onResume"), "foreground = true", "!liveAuthorization()", "cancelAndFinish()")
    }

    @Test
    fun resultSerializationHasNoIdentityCredentialOrStorageChannel() {
        val activity = source("EskPlatformSnapshotConsentActivity.kt")
        val view = source("EskPlatformSnapshotConsentView.kt")
        val wire = source("EskPlatformSnapshotWire.kt")
        listOf("Log.", "printStackTrace(", "println(", "ClipboardManager", "WebView",
            "addJavascriptInterface(", "evaluateJavascript(", "openFileOutput(", "FileOutputStream(",
            "getSharedPreferences(", "getPreferences(", "putParcelable(", "putSerializable(",
        ).forEach { assertFalse("Unexpected output/storage path: $it", (activity + view + wire).contains(it)) }
        assertFalse(activity.contains("putExtra"))
        assertFalse(view.contains("putExtra"))
        val project = method(wire, "composeEskPlatformSnapshot")
        listOf("account.userId", "account.displayName", "account.entries", "account.updatedAt", "session.",
            "\"token\"", "\"user_id\"", "\"nickname\"", "\"available\"", "\"revision\"",
        ).forEach { assertFalse("Unexpected formal summary field: $it", project.contains(it)) }
        val result = method(wire, "eskPlatformSnapshotResult")
        ordered(result, "EskPlatformSnapshotContract.validSnapshot(fields, nonce, startedAt, now)",
            "val extras = Bundle()", "EskPlatformSnapshotContract.KEYS.forEach",
            "extras.putString(it, fields.getValue(it))", "return Intent().putExtras(extras)")
    }

    @Test
    fun consentViewUsesPreviewResourceAndClearsIdentityAndClickAuthorization() {
        val view = source("EskPlatformSnapshotConsentView.kt")
        assertTrue(view.contains("inflate(R.layout.esk_platform_consent_preview, null)"))
        assertTrue(view.contains("IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS"))
        assertTrue(view.contains("disableState(root)"))
        ordered(method(view, "clear"), "account.text = \"尚未确认账户\"", "primary.isEnabled = false",
            "primary.setOnClickListener(null)")
        ordered(method(view, "loading"), "primary.isEnabled = false", "primary.setOnClickListener(null)")
        ordered(method(view, "unavailable"), "clear()", "status.text = message")
        ordered(method(view, "show"), "Character.FORMAT", "take(64)", "primary.isEnabled = true",
            "primary.setOnClickListener { confirm() }")
        ordered(method(view, "disableState"), "view.isSaveEnabled = false", "view.isSaveFromParentEnabled = false",
            "view.setTextIsSelectable(false)", "disableState(view.getChildAt(i))")
        assertTrue(view.contains("button.filterTouchesWhenObscured = true"))
        assertTrue(view.contains("MotionEvent.FLAG_WINDOW_IS_OBSCURED or"))
        assertTrue(view.contains("MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED"))
        assertTrue(view.contains("style(primary, true)"))

        val layoutText = read("android/app/src/main/res/layout/esk_platform_consent_preview.xml")
        val layout = xml(layoutText)
        assertEquals("ScrollView", layout.documentElement.tagName)
        val buttons = layout.getElementsByTagName("Button")
        assertEquals(2, buttons.length)
        (0 until buttons.length).forEach {
            val button = buttons.item(it) as Element
            assertEquals("52dp", android(button, "minHeight"))
            assertEquals("wrap_content", android(button, "layout_height"))
            assertFalse(android(button, "singleLine") == "true")
            if (android(button, "id").endsWith("esk_platform_consent_primary_action"))
                assertEquals("false", android(button, "enabled"))
        }
        listOf("确认并读取正式摘要", "尚未上链", "不包含 Paper 模拟资产", "不传登录凭据、身份或流水",
            "不是实时余额", "返回后无法远程撤回", "本次不会买卖、转账或授权交易",
        ).forEach { assertTrue("Missing disclosure: $it", layoutText.contains(it)) }
    }

    @Test
    fun webRemainsExplanationOnlyAndPaperKeepsItsIndependentSeventeenFieldPath() {
        val web = read("server/src/assets/web_page.html")
        val entry = Regex("<section\\b[^>]*id=\"profileEskPlatformEntry\"[^>]*>[\\s\\S]*?</section>")
            .findAll(web).toList()
        assertEquals(1, entry.size)
        listOf("本网页暂不读取正式私有余额", "正式摘要授权仅供原生 APK", "量化接收端仍待接入",
            "不会自动绑定网页账户", "href=\"/app/ElonSpeed-latest.apk\"",
        ).forEach { assertTrue(entry.single().value.contains(it)) }
        listOf("READ_ESK_PLATFORM_SNAPSHOT", "platform_android_snapshot.v1", "/api/me/assets/esk/platform")
            .forEach { assertFalse(web.contains(it)) }
        assertEquals("yilong.esk.android_snapshot.v1", EskSnapshotContract.PROTOCOL)
        assertEquals("com.elon.app.action.READ_ESK_SNAPSHOT", EskSnapshotContract.ACTION)
        assertEquals(17, EskSnapshotContract.KEYS.size)
        assertFalse(EskSnapshotContract.KEYS.contains("source"))
        val paper = kotlin("esk/handoff/EskSnapshotConsentActivity.kt") +
            kotlin("esk/handoff/EskSnapshotCaller.kt") + kotlin("esk/handoff/EskSnapshotHttpsReader.kt")
        assertTrue(paper.contains("com.elon.quant.assets.EskAssetsActivity"))
        assertTrue(paper.contains("encodedPath(\"/api/me/assets/esk\")"))
        assertFalse(paper.contains("EskPlatformSnapshot"))
        assertFalse(paper.contains("/api/me/assets/esk/platform"))
    }

    private fun source(name: String) = kotlin("esk/platform/handoff/$name")
    private fun kotlin(relative: String) = read("android/app/src/main/kotlin/com/elon/app/$relative")
    private fun android(element: Element, name: String) = element.getAttributeNS(ANDROID_NS, name)

    private fun xml(source: String) = DocumentBuilderFactory.newInstance().apply {
        isNamespaceAware = true
        setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
    }.newDocumentBuilder().parse(ByteArrayInputStream(source.toByteArray(StandardCharsets.UTF_8)))

    private fun ordered(source: String, vararg markers: String) {
        var previous = -1
        markers.forEach { marker ->
            val index = source.indexOf(marker, previous + 1)
            assertTrue("Missing or out-of-order source step: $marker", index > previous)
            previous = index
        }
    }

    /** Selected source methods have block bodies with balanced braces, including string templates. */
    private fun method(source: String, name: String): String {
        val signature = Regex("\\bfun\\s+${Regex.escape(name)}\\s*\\(").find(source)
            ?: error("Missing source method $name")
        val start = source.indexOf('{', signature.range.last + 1)
        check(start >= 0) { "Missing source body $name" }
        var depth = 0
        for (index in start until source.length) {
            if (source[index] == '{') depth++
            if (source[index] == '}' && --depth == 0) return source.substring(start, index + 1)
        }
        error("Unclosed source body $name")
    }

    private fun read(relative: String): String =
        String(Files.readAllBytes(root().resolve(relative)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }.take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Repository root unavailable for source wiring checks")
    }

    companion object {
        private const val ANDROID_NS = "http://schemas.android.com/apk/res/android"
        private const val PROVIDER = ".esk.platform.handoff.EskPlatformSnapshotConsentActivity"
    }
}
