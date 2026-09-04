package com.elon.app.esk.platform

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

/** Source wiring regression checks, not Android lifecycle or device acceptance evidence. */
class EskPlatformWiringTest {
    @Test
    fun saveAndClearRotateRevisionWithinTheSameSessionEdit() {
        val auth = kotlin("AuthManager.kt")
        listOf("saveSession", "clear").forEach { name ->
            ordered(method(auth, name), "prefs(ctx).edit().apply {",
                "putString(\"auth_session_revision\", UUID.randomUUID().toString())", "}.apply()")
        }
    }

    @Test
    fun nativeFormalPageIsNonExportedAndDoesNotReturnPrivateData() {
        val manifest = read("android/app/src/main/AndroidManifest.xml")
        val xml = DocumentBuilderFactory.newInstance().apply {
            isNamespaceAware = true
            setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
        }.newDocumentBuilder().parse(ByteArrayInputStream(manifest.toByteArray(StandardCharsets.UTF_8)))
        val android = "http://schemas.android.com/apk/res/android"
        val activities = xml.getElementsByTagName("activity")
        val matches = (0 until activities.length).map { activities.item(it) as org.w3c.dom.Element }
            .filter { it.getAttributeNS(android, "name") == ".esk.platform.EskPlatformAssetsActivity" }
        assertEquals(1, matches.size)
        assertEquals("false", matches.single().getAttributeNS(android, "exported"))
        assertEquals("false", matches.single().getAttributeNS(android, "allowTaskReparenting"))
        assertEquals("standard", matches.single().getAttributeNS(android, "launchMode"))
        assertEquals(0, matches.single().getElementsByTagName("intent-filter").length)
        val aliases = xml.getElementsByTagName("activity-alias")
        (0 until aliases.length).forEach { index ->
            val target = (aliases.item(index) as org.w3c.dom.Element).getAttributeNS(android, "targetActivity")
            assertFalse(target.endsWith("EskPlatformAssetsActivity"))
        }

        val activity = kotlin("esk/platform/EskPlatformAssetsActivity.kt")
        val create = method(activity, "onCreate")
        ordered(create, "FLAG_SECURE", "EskPlatformAssetsView(this, ::finish, ::refresh)")
        listOf("AuthManager", "EskPlatformSessionStore", ".capture(").forEach { assertFalse(create.contains(it)) }
        listOf("setResult(", "putExtra(", "getIntent(", "intent.", "WebView", "ClipboardManager",
            "SharedPreferences", "openFileOutput(", "Log.", "printStackTrace(",
        ).forEach { assertFalse("Private page acquired an output channel: $it", activity.contains(it)) }
    }

    @Test
    fun activityChecksTransportBeforeSessionAndChecksFreshSessionBeforeDisplay() {
        val activity = kotlin("esk/platform/EskPlatformAssetsActivity.kt")
        val refresh = method(activity, "refresh")
        ordered(refresh, "clearPrivateState()", "eskPlatformEndpoint(BuildConfig.SERVER_URL) == null",
            "return", "EskPlatformSessionStore(this)", "sessions.capture()", "gate.begin(session,",
            "EskPlatformAccountReader()", "source.fetch(BuildConfig.SERVER_URL) { session.token }",
            "if (reader !== source || !foreground || isFinishing || isDestroyed)",
            "gate.consume(ticket, sessions.capture()", "invalidateDisplayedAccount()",
            "return@runOnUiThread", "page.show(account, session.displayName)")
        val guardStart = refresh.indexOf("if (eskPlatformEndpoint")
        val guardEnd = refresh.indexOf('}', guardStart)
        assertTrue(refresh.substring(guardStart, guardEnd).contains("return"))
        assertTrue(refresh.contains("gate.invalidate()"))
        assertTrue(refresh.contains("reader?.cancel()"))
        assertTrue(refresh.contains("runOnUiThread { invalidateDisplayedAccount() }"))
        assertTrue(refresh.contains("EskPlatformRequestGate.MAX_REQUEST_MS"))
        assertTrue(refresh.contains("minOf(untilExpiry, 60_000L)"))
        assertFalse(refresh.contains("page.show(" + "EskPlatformAccount("))
    }

    @Test
    fun lifecycleInvalidatesRequestsAndClearsPrivateViewsWithoutSavingBalances() {
        val activity = kotlin("esk/platform/EskPlatformAssetsActivity.kt")
        ordered(method(activity, "clearPrivateState"), "gate.invalidate()", "reader?.cancel()",
            "reader = null", "store?.close()", "store = null", "handler.removeCallbacksAndMessages(null)", "page.clear()")
        ordered(method(activity, "onResume"), "foreground = true", "refresh()")
        listOf("onPause", "onDestroy").forEach { name ->
            ordered(method(activity, name), "foreground = false", "clearPrivateState()", "super.$name()")
        }
        ordered(method(activity, "onStop"), "clearPrivateState()", "super.onStop()")
        ordered(method(activity, "onSaveInstanceState"), "clearPrivateState()", "super.onSaveInstanceState(outState)", "outState.clear()")
        ordered(method(activity, "invalidateDisplayedAccount"), "clearPrivateState()", "if (foreground", "page.unavailable(")

        val view = kotlin("esk/platform/EskPlatformAssetsView.kt")
        ordered(method(view, "clear"), "total.text = \"— ESK\"", "accountLabel.text = \"当前账户\"", "entries.removeAllViews()")
        assertTrue(method(view, "loading").contains("clear()"))
        assertTrue(method(view, "unavailable").contains("clear()"))
        assertTrue(view.contains("IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS"))
        assertTrue(view.contains("disableState(root)"))
        ordered(method(view, "disableState"), "view.isSaveEnabled = false", "view.isSaveFromParentEnabled = false", "disableState(view.getChildAt(index))")
        val show = method(view, "show")
        assertTrue(show.contains("isSaveEnabled = false"))
        assertTrue(show.contains("isSaveFromParentEnabled = false"))
        assertTrue(show.contains("setTextIsSelectable(false)"))
    }

    @Test
    fun profileEntrySurvivesPaperCardRemountWithoutDuplicatingOrFetching() {
        val profile = kotlin("MainProfileQuickActions.kt")
        ordered(method(profile, "refreshProfileSummary"), "eskAssetCard.attachAndRefresh()",
            "EskPlatformProfileEntry.attach(activity, binding)")
        val entry = kotlin("esk/platform/EskPlatformProfileEntry.kt")
        assertTrue(entry.contains("private const val ENTRY_TAG = \"esk-platform-profile-entry\""))
        ordered(method(entry, "attach"), "binding.profileEskAssetContainer",
            "if (host.findViewWithTag<View>(ENTRY_TAG) != null) return", "host.addView(", "tag = ENTRY_TAG")
        assertTrue(entry.contains("Intent(activity, EskPlatformAssetsActivity::class.java)"))
        listOf("AuthManager", "SessionStore", "AccountReader", "putExtra(", "fetch(", "removeAllViews(")
            .forEach { assertFalse("Entry must remain static: $it", entry.contains(it)) }
        val layout = read("android/app/src/main/res/layout/activity_main.xml")
        ordered(layout, "@+id/profileAssetsTitle", "@+id/profileEskAssetContainer", "@+id/profileUsageContainer")
    }

    @Test
    fun nativeViewShowsCompleteTotalAndWebExplainsNativeOnlyFormalSource() {
        val view = kotlin("esk/platform/EskPlatformAssetsView.kt")
        val show = method(view, "show")
        assertTrue(show.contains("total.text = \"${'$'}{account.total} ESK\""))
        assertTrue(show.contains("account.entries.isEmpty()"))
        assertTrue(show.contains("暂无正式登记"))
        assertTrue(show.contains("account.historyHasMore"))
        assertTrue(show.contains("数量来自全部已审核账本"))
        assertFalse(show.contains("sumOf("))
        assertFalse(show.contains("EskAssetApi"))
        val layout = read("android/app/src/main/res/layout/esk_platform_assets_preview.xml")
        listOf("正式平台登记 · 尚未上链", "不包含 Paper 模拟余额", "最近审核入账", "不代表系统已自动验证付款",
            "正式卖回结算暂未开放", "模拟卖回不适用于此数量").forEach { assertTrue(layout.contains(it)) }

        val web = read("server/src/assets/web_page.html")
        val section = Regex("<section\\b[^>]*id=\"profileEskPlatformEntry\"[^>]*>[\\s\\S]*?</section>")
            .findAll(web).toList()
        assertEquals(1, section.size)
        listOf("本网页暂不读取正式私有余额", "Paper 模拟资产不包含正式登记数量", "当前 HTTP 不可用",
            "href=\"/app/ElonSpeed-latest.apk\"").forEach { assertTrue(section.single().value.contains(it)) }
        assertFalse(section.single().value.contains("<script"))
        assertTrue(section.single().value.contains("<a class=\"profile-row\""))
        assertTrue(web.contains("min-height: 76px;"))
        ordered(web, "id=\"profileEskTotal\"", "id=\"profileEskPlatformEntry\"", "<h3>Token 额度</h3>")
        assertFalse(web.contains("/api/me/assets/esk/platform"))
        assertFalse(web.contains("yilong.esk.platform_account.v1"))
        assertTrue(web.contains("api('/api/me/assets/esk', { cache: 'no-store' })"))
    }

    @Test
    fun privateReaderSelectsOnlyFixedHttpsPlatformPathBeforeReadingToken() {
        val reader = kotlin("esk/platform/EskPlatformAccountReader.kt")
        val fetch = method(reader, "fetch")
        ordered(fetch, "eskPlatformEndpoint(configuredBase)", "tokenProvider()", "Request.Builder()", "calls.newCall(request)")
        assertTrue(reader.contains("encodedPath(\"/api/me/assets/esk/platform\")"))
        assertTrue(reader.contains("addQueryParameter(\"limit\", \"20\")"))
        assertTrue(reader.contains("uri.scheme == \"https\""))
        assertTrue(reader.contains("uri.rawUserInfo == null"))
        assertTrue(reader.contains("uri.rawQuery == null"))
        assertTrue(reader.contains("uri.rawFragment == null"))
        listOf(".followRedirects(false)", ".followSslRedirects(false)",
            ".retryOnConnectionFailure(false)", ".cache(null)",
            "CookieJar.NO_COOKIES", "Proxy.NO_PROXY", "EskPlatformAccountParser.parse(bytes)",
        ).forEach { assertTrue("Missing reader boundary: $it", reader.contains(it)) }
        assertFalse(reader.contains("addQueryParameter(\"user"))
        assertFalse(reader.contains(".post("))
        assertFalse(reader.contains(".sslSocketFactory("))
        assertFalse(reader.contains(".hostnameVerifier("))
        assertFalse(reader.contains(".addInterceptor("))
    }

    @Test
    fun originalPaperApiAndActionsRemainOnTheirOwnSource() {
        val api = kotlin("esk/EskAssetApi.kt")
        val card = kotlin("esk/EskAssetCard.kt")
        assertTrue(api.contains("yilong.esk.asset_account.v2"))
        assertTrue(api.contains("url(\"/api/me/assets/esk\")"))
        assertTrue(api.contains("root.optBoolean(\"simulated\")"))
        assertFalse(api.contains("/api/me/assets/esk/platform"))
        assertFalse(api.contains("yilong.esk.platform_account.v1"))
        assertTrue(card.contains("value.mode == \"paper\""))
        assertTrue(card.contains("EskSellbackDialog(activity, api, current, ::refresh)"))
        assertTrue(card.contains("EskPaperExchangeDialog(activity, api, ::refresh)"))
    }

    @Test
    fun originalSeventeenFieldIpcCannotCarryFormalPlatformBalances() {
        val nonce = "a".repeat(64)
        val paper = mapOf(
            "protocol" to EskSnapshotContract.PROTOCOL,
            "nonce" to nonce,
            "asset_id" to "esk",
            "symbol" to "ESK",
            "mode" to "paper",
            "issuance_mode" to "paper_recorded",
            "chain_status" to "not_deployed",
            "simulated" to "true",
            "funds_moved" to "false",
            "total" to "1.000000",
            "available" to "1.000000",
            "reserved_for_sellback" to "0.000000",
            "reserved_for_quant" to "0.000000",
            "reserved_total" to "0.000000",
            "revision" to "1",
            "observed_elapsed_ms" to "2000",
            "expires_elapsed_ms" to "62000",
        )
        fun accepts(fields: Map<String, String>) =
            EskSnapshotContract.validSnapshot(fields, nonce, 1000L, 3000L)
        assertEquals("yilong.esk.android_snapshot.v1", EskSnapshotContract.PROTOCOL)
        assertEquals(17, EskSnapshotContract.KEYS.size)
        assertEquals(paper.keys, EskSnapshotContract.KEYS)
        assertTrue(accepts(paper))
        assertFalse(accepts(paper + ("mode" to "platform_recorded")))
        assertFalse(accepts(paper + ("issuance_mode" to "platform_recorded")))
        assertFalse(accepts(paper + ("simulated" to "false")))
        assertFalse(accepts(paper + ("source" to "platform_recorded")))

        val reader = kotlin("esk/handoff/EskSnapshotHttpsReader.kt")
        val parser = kotlin("esk/handoff/EskSnapshotAccountParser.kt")
        assertTrue(reader.contains("encodedPath(\"/api/me/assets/esk\")"))
        assertFalse(reader.contains("/api/me/assets/esk/platform"))
        assertTrue(parser.contains("yilong.esk.asset_account.v2"))
        assertFalse(parser.contains("yilong.esk.platform_account.v1"))
    }

    private fun kotlin(relative: String): String =
        read("android/app/src/main/kotlin/com/elon/app/$relative")

    private fun ordered(source: String, vararg markers: String) {
        var previous = -1
        markers.forEach { marker ->
            val index = source.indexOf(marker, previous + 1)
            assertTrue("Missing or out-of-order source step: $marker", index > previous)
            previous = index
        }
    }

    /** These selected methods have block bodies; fail if a method is renamed or disappears. */
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
            ?: error("Repository root unavailable for source wiring tests")
    }
}
