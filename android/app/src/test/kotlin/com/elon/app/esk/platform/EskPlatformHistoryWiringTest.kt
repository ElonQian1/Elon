package com.elon.app.esk.platform

import java.io.ByteArrayInputStream
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.*
import org.junit.Test

/** Static wiring proof only; actual Android lifecycle and pixels remain separate acceptance. */
class EskPlatformHistoryWiringTest {
    @Test fun historyIsAnExplicitNativeNonExportedDestination() {
        val doc = DocumentBuilderFactory.newInstance().apply {
            isNamespaceAware = true
            setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
        }.newDocumentBuilder().parse(ByteArrayInputStream(read("android/app/src/main/AndroidManifest.xml").toByteArray()))
        val ns = "http://schemas.android.com/apk/res/android"
        val nodes = doc.getElementsByTagName("activity")
        val matches = (0 until nodes.length).map { nodes.item(it) as org.w3c.dom.Element }
            .filter { it.getAttributeNS(ns, "name") == ".esk.platform.EskPlatformHistoryActivity" }
        assertEquals(1, matches.size)
        assertEquals("false", matches.single().getAttributeNS(ns, "exported"))
        assertEquals("false", matches.single().getAttributeNS(ns, "allowTaskReparenting"))
        assertEquals(0, matches.single().getElementsByTagName("intent-filter").length)
        val aliases = doc.getElementsByTagName("activity-alias")
        (0 until aliases.length).forEach {
            assertFalse((aliases.item(it) as org.w3c.dom.Element).getAttributeNS(ns, "targetActivity")
                .endsWith("EskPlatformHistoryActivity"))
        }
        assertTrue(source("EskPlatformAssetsActivity").contains("Intent(this, EskPlatformHistoryActivity::class.java)"))
        assertTrue(source("EskPlatformAssetsView").contains("R.id.esk_platform_history_open"))
        assertTrue(read("android/app/src/main/res/layout/esk_platform_assets_preview.xml").contains("查看完整审核流水"))
    }

    @Test fun privateHistoryHasNoExportOrPersistenceChannel() {
        val activity = source("EskPlatformHistoryActivity")
        listOf("setResult(", "putExtra(", "getIntent(", "intent.", "WebView", "SharedPreferences",
            "ClipboardManager", "openFileOutput(", "Log.", "printStackTrace(")
            .forEach { assertFalse(it, activity.contains(it)) }
        ordered(activity, "FLAG_SECURE", "EskPlatformHistoryView(this,")
        ordered(method(activity, "refresh"), "clearPrivateState()", "eskPlatformEndpoint(BuildConfig.SERVER_URL)",
            "return", "EskPlatformSessionStore(this)", "sessions.capture()", "history.first(session)")
        ordered(method(activity, "next"), "reader != null", "sessions.capture()", "history.next(session,", "load(ticket,")
    }

    @Test fun freshIdentityAndBothRequestGatesPrecedeDisplay() {
        val activity = source("EskPlatformHistoryActivity")
        ordered(method(activity, "load"), "gate.begin(session,", "page.loading()", "MAX_REQUEST_MS",
            "source.fetch(BuildConfig.SERVER_URL, historyTicket.cursor)", "if (reader !== source || !foreground",
            "sessions.capture()", "gate.consume(ticket, current", "history.accept(historyTicket, records, current",
            "page.show(records, session.displayName)", "minOf(untilExpiry,")
        assertTrue(activity.contains("EskPlatformHistoryReadFailure.HISTORY_CHANGED"))
        assertTrue(activity.contains("账本已更新，请重新加载"))
        ordered(method(activity, "clearPrivateState"), "gate.invalidate()", "history.clear()", "reader?.cancel()",
            "store?.close()", "handler.removeCallbacksAndMessages(null)", "page.clear()")
        listOf("onPause", "onStop", "onDestroy", "onSaveInstanceState").forEach {
            assertTrue(method(activity, it).contains("clearPrivateState()"))
        }
        assertTrue(method(activity, "onSaveInstanceState").contains("outState.clear()"))
    }

    @Test fun pageUsesWholeAccountTotalAndOnlyCurrentRange() {
        val view = source("EskPlatformHistoryView")
        assertTrue(view.contains("${'$'}{page.total} ESK"))
        assertTrue(view.contains("${'$'}{page.rangeStart}–${'$'}{page.rangeEnd}"))
        assertTrue(view.contains("${'$'}{page.entryCount}"))
        assertFalse(view.contains("sumOf("))
        assertTrue(method(view, "clear").contains("entries.removeAllViews()"))
        assertTrue(method(view, "show").contains("clear()"))
        listOf("IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS", "disableState(root)", "isSaveEnabled = false",
            "isSaveFromParentEnabled = false", "setTextIsSelectable(false)", "filterTouchesWhenObscured = true",
            "FLAG_WINDOW_IS_PARTIALLY_OBSCURED").forEach { assertTrue(it, view.contains(it)) }
        val layout = read("android/app/src/main/res/layout/esk_platform_history_preview.xml")
        listOf("全账户审核总额", "不是当前页小计", "不包含 Paper 模拟资产", "尚未上链", "下一页",
            "重新加载", "@color/elon_bg_app", "android:minHeight=\"52dp\"").forEach { assertTrue(it, layout.contains(it)) }
        assertFalse(layout.contains("0.000000"))
    }

    @Test fun webDisclosesNativeHistoryWithoutFetchingCredentialsOrClaimingAvailability() {
        val section = Regex("<section\\b[^>]*id=\"profileEskPlatformEntry\"[^>]*>[\\s\\S]*?</section>")
            .find(read("server/src/assets/web_page.html"))!!.value
        listOf("查看完整审核流水", "本网页暂不读取正式私有余额", "当前 HTTP 不可用",
            "href=\"/app/ElonSpeed-latest.apk\"").forEach { assertTrue(it, section.contains(it)) }
        assertFalse(section.contains("/api/me/assets/esk/platform/history"))
    }

    private fun source(name: String) = read("android/app/src/main/kotlin/com/elon/app/esk/platform/$name.kt")
    private fun ordered(source: String, vararg markers: String) {
        var position = -1
        markers.forEach { val next = source.indexOf(it, position + 1); assertTrue(it, next > position); position = next }
    }
    private fun method(source: String, name: String): String {
        val match = Regex("\\bfun\\s+${Regex.escape(name)}\\s*\\(").find(source) ?: error("Missing $name")
        val start = source.indexOf('{', match.range.last + 1)
        var depth = 0
        for (index in start until source.length) {
            if (source[index] == '{') depth++
            if (source[index] == '}' && --depth == 0) return source.substring(start, index + 1)
        }
        error("Unclosed $name")
    }
    private fun read(path: String) = String(Files.readAllBytes(root().resolve(path)), Charsets.UTF_8)
    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath()) { it.parent }
        .take(6).first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
}
