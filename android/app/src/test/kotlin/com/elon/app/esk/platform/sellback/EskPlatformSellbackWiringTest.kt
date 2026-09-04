package com.elon.app.esk.platform.sellback

import com.elon.eskcontract.EskPlatformProgressContract
import java.io.ByteArrayInputStream
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.*
import org.junit.Test

/** Source wiring regression only, not Android lifecycle, device, server, or visual acceptance. */
class EskPlatformSellbackWiringTest {
    @Test fun separateNonExportedActivityHasOnlyAnExplicitNativeEntry() {
        val doc = xml("android/app/src/main/AndroidManifest.xml")
        val ns = "http://schemas.android.com/apk/res/android"
        val nodes = doc.getElementsByTagName("activity")
        val matches = (0 until nodes.length).map { nodes.item(it) as org.w3c.dom.Element }
            .filter { it.getAttributeNS(ns, "name") == ".esk.platform.sellback.EskPlatformSellbackActivity" }
        assertEquals(1, matches.size)
        val activity = matches.single()
        assertEquals("false", activity.getAttributeNS(ns, "exported"))
        assertEquals("false", activity.getAttributeNS(ns, "allowTaskReparenting"))
        assertEquals("standard", activity.getAttributeNS(ns, "launchMode"))
        assertEquals(0, activity.getElementsByTagName("intent-filter").length)
        val aliases = doc.getElementsByTagName("activity-alias")
        for (index in 0 until aliases.length) assertFalse((aliases.item(index) as org.w3c.dom.Element)
            .getAttributeNS(ns, "targetActivity").endsWith("EskPlatformSellbackActivity"))
        val source = platform("EskPlatformAssetsActivity")
        assertTrue(source.contains("page.attachSellback"))
        assertTrue(source.contains("Intent(this, com.elon.app.esk.platform.sellback.EskPlatformSellbackActivity::class.java)"))
        assertFalse(source.contains("putExtra("))
        assertTrue(platform("EskPlatformAssetsView").contains("R.id.esk_platform_sellback_open"))
        assertTrue(read("android/app/src/main/res/layout/esk_platform_assets_preview.xml").contains("卖回申请与占用"))
    }

    @Test fun sourceGuardPrecedesEverySessionAdapterAndPrivateNetworkRequest() {
        val activity = source("Activity")
        ordered(method(activity, "onCreate"), "FLAG_SECURE", "EskPlatformSellbackView(")
        ordered(method(activity, "session"), "eskPlatformEndpoint(BuildConfig.SERVER_URL)", "return null",
            "EskPlatformSessionStore(this)", "sessions?.capture()", "reviewIdentity?.belongsTo(captured)")
        val client = source("Client")
        for (name in listOf("page", "lookup", "lookupKey", "execute"))
            ordered(method(client, name), "endpoint(base)", "exchange(")
        assertTrue(client.contains("= newEskPlatformClient()"))
        assertTrue(client.contains("?.query(null)"))
        val shared = platform("EskPlatformAccountReader")
        for (guard in listOf("followRedirects(false)", "followSslRedirects(false)", "retryOnConnectionFailure(false)",
            "cookieJar(CookieJar.NO_COOKIES)", "proxy(Proxy.NO_PROXY)", "callTimeout(15, TimeUnit.SECONDS)"))
            assertTrue(guard, shared.contains(guard))
    }

    @Test fun confirmationAndCurrentSessionGatePrecedeWriteAndRendering() {
        val activity = source("Activity")
        ordered(method(activity, "submit"), "freshDisplay()", "blocked(session)", "SellbackAction.submit(",
            "state.prepare(", "confirm(draft, false)")
        ordered(method(activity, "cancel"), "freshDisplay()", "records.none", "SellbackAction.cancel(", "state.prepare(")
        ordered(method(activity, "confirm"), "view.confirm(", "val current = session()", "state.confirm(",
            "EskPlatformSellbackRecovery.remember(", "source.execute(")
        ordered(method(activity, "request"), "gate.begin(session,", "handler.removeCallbacksAndMessages(null)",
            "view.loading()", "MAX_REQUEST_MS", "adapter.capture()", "session.sameAs(current)", "requireNotNull(current).token",
            "client !== source || !foreground", "adapter.capture()", "gate.consume(ticket, current", "accepted(it)")
        ordered(method(activity, "failed"), "source.cancel()", "state.unknown(mutation)", "SIGN_IN_REQUIRED", "UNKNOWN")
        assertFalse(method(activity, "failed").contains("UUID"))
        assertFalse(method(activity, "failed").contains("complete("))
        assertTrue(method(activity, "retry").contains("state.retry("))
    }

    @Test fun lifecycleErasesDisplayWritePayloadSessionAndSavedState() {
        val activity = source("Activity")
        for (methodName in listOf("onPause", "onStop", "onSaveInstanceState", "onDestroy"))
            assertTrue(methodName, method(activity, methodName).contains("clearPrivateState()"))
        assertTrue(method(activity, "onSaveInstanceState").contains("outState.clear()"))
        ordered(method(activity, "clearPrivateState"), "gate.invalidate()", "client?.cancel()", "state.clear()",
            "sessions?.close()", "owner = null", "handler.removeCallbacksAndMessages(null)", "clearDisplay()")
        val clear = method(activity, "clearDisplay")
        for (part in listOf("summary = null", "records = emptyList()", "position = null", "shownAt = -1L", "view.clear()"))
            assertTrue(part, clear.contains(part))
        assertTrue(method(activity, "onResume").contains("refresh(true)"))
        assertTrue(method(activity, "refresh").contains("source.lookupKey("))
        assertTrue(method(activity, "invalidateSession").contains("EskPlatformSellbackRecovery.clear()"))
        assertTrue(method(activity, "invalidateSession").contains("reviewIdentity = null"))
    }

    @Test fun noCredentialExportPersistenceLoggingPaperOrWebBridge() {
        val names = listOf("Activity", "View", "State", "Recovery", "Model", "Parser", "Client")
        for (name in names) {
            val text = source(name)
            for (forbidden in listOf("putExtra(", "setResult(", "getIntent(", "intent.", "WebView(", "JavascriptInterface",
                "ClipboardManager", "getSharedPreferences(", "openFileOutput(", "Log.", "println(", "printStackTrace(",
                "EskAssetApi", "EskSellbackDialog", "EskPlatformSnapshotContract", "EskSnapshotContract"))
                assertFalse("$name/$forbidden", text.contains(forbidden))
        }
        val recovery = source("Recovery")
        assertTrue(recovery.contains("session.revision?.let"))
        assertFalse(recovery.contains("session.token")); assertFalse(recovery.contains("action.body"))
        assertFalse(recovery.contains("action.amount")); assertFalse(recovery.contains("action.terms"))
        val lookup = method(source("Client"), "lookupKey")
        assertTrue(lookup.contains("yilong.esk.platform_sellback_lookup.v1"))
        assertFalse(lookup.contains("addQueryParameter")); assertFalse(lookup.contains("addPathSegment(key)"))
    }

    @Test fun displayedNumbersComeOnlyFromSameServerSnapshotAndMemoryDoesNotAccumulate() {
        val view = source("View")
        for (field in listOf("summary.total", "summary.reserved", "summary.available"))
            assertTrue(view.contains("sellbackAmount($field)"))
        assertFalse(view.contains("sumOf("))
        val activity = source("Activity")
        assertTrue(method(activity, "accept").contains("summary = page.summary"))
        assertTrue(activity.contains("summary = result.summary"))
        val next = method(activity, "next")
        for (guard in listOf("previous.summary", "previous.end", "previous.requests.lastOrNull()",
            "it.summary != expected", "it.start != end + 1", "last.created > first.created")) assertTrue(guard, next.contains(guard))
        assertFalse(activity.contains("addAll(")); assertFalse(activity.contains("summary.copy("))
        assertTrue(method(activity, "freshDisplay").contains("now - shownAt >= DISPLAY_MS"))
        assertTrue(method(activity, "armExpiry").contains("state.clear()"))
    }

    @Test fun previewAndConfirmationRemainAccessibleSafePlainTextAndNonPersistent() {
        val view = source("View")
        for (guard in listOf("FLAG_SECURE", "disableState(root)", "disableState(content)",
            "IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS", "FLAG_WINDOW_IS_PARTIALLY_OBSCURED", "button.isEnabled = false",
            "consent.isChecked && independent.isChecked", "setTextIsSelectable(false)", "eraseText(content)",
            "isSaveEnabled = false", "isSaveFromParentEnabled = false")) assertTrue(guard, view.contains(guard))
        val layout = read("android/app/src/main/res/layout/esk_platform_sellback_preview.xml")
        for (copy in listOf("正式总量", "申请占用", "可申请量", "尚未上链", "申请不等于成交", "@color/elon_bg_app"))
            assertTrue(copy, layout.contains(copy))
        val doc = xml("android/app/src/main/res/layout/esk_platform_sellback_preview.xml")
        val ns = "http://schemas.android.com/apk/res/android"
        val buttons = doc.getElementsByTagName("Button")
        for (index in 0 until buttons.length) assertEquals("52dp", (buttons.item(index) as org.w3c.dom.Element).getAttributeNS(ns, "minHeight"))
        assertFalse(layout.contains("0.000000")); assertFalse(layout.contains("WebView"))
    }

    @Test fun account18RemainsIndependentFromReadOnlyProgressAndItsRequestCounts() {
        assertEquals(35, EskPlatformProgressContract.TOP_KEYS.size)
        for (field in listOf("idempotency_key", "new_requests_enabled", "entry_count", "policy"))
            assertFalse(field in EskPlatformProgressContract.TOP_KEYS)
        val parser = platform("EskPlatformAccountParser")
        val keys = Regex("private val rootKeys = setOf\\(([\\s\\S]*?)\\)").find(parser)!!.groupValues[1]
        assertEquals(18, Regex("\"[^\"]+\"").findAll(keys).count())
        assertTrue(parser.contains("capabilities.values.all { it == false }"))
        val progress = read("android/app/src/main/kotlin/com/elon/eskcontract/EskPlatformProgressContract.kt")
        for (capability in listOf("sellback_settlement", "submit_request", "cancel_request"))
            assertTrue(progress.contains("\"$capability\" to \"false\""))
    }

    private fun source(suffix: String) = read("android/app/src/main/kotlin/com/elon/app/esk/platform/sellback/EskPlatformSellback$suffix.kt")
    private fun platform(name: String) = read("android/app/src/main/kotlin/com/elon/app/esk/platform/$name.kt")
    private fun xml(path: String) = DocumentBuilderFactory.newInstance().apply {
        isNamespaceAware = true; setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
    }.newDocumentBuilder().parse(ByteArrayInputStream(read(path).toByteArray()))
    private fun ordered(text: String, vararg markers: String) {
        var index = -1
        for (marker in markers) { val next = text.indexOf(marker, index + 1); assertTrue(marker, next > index); index = next }
    }
    private fun method(text: String, name: String): String {
        val match = Regex("\\bfun(?:\\s+<[^>]+>)?\\s+${Regex.escape(name)}\\s*\\(").find(text) ?: error("Missing $name")
        val start = text.indexOf('{', match.range.last + 1)
        var depth = 0
        for (index in start until text.length) {
            if (text[index] == '{') depth++
            if (text[index] == '}' && --depth == 0) return text.substring(start, index + 1)
        }
        error("Unclosed $name")
    }
    private fun read(path: String) = String(Files.readAllBytes(root().resolve(path)), Charsets.UTF_8)
    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath()) { it.parent }
        .take(6).first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
}
