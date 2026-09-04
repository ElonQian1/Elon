package com.elon.app.esk.platform.progress

import java.io.ByteArrayInputStream
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.*
import org.junit.Test
import org.w3c.dom.Element

/** Source/manifest retirement checks, not device, backend execution or a balance migration. */
class EskPlatformNativeRetirementTest {
    @Test fun obsoleteBridgesContractsFixturesLayoutAndDedicatedScriptAreAbsent() {
        val old = listOf(
            "main/kotlin/com/elon/app/esk/handoff/EskSnapshotAccountParser.kt",
            "main/kotlin/com/elon/app/esk/handoff/EskSnapshotCaller.kt",
            "main/kotlin/com/elon/app/esk/handoff/EskSnapshotConsentActivity.kt",
            "main/kotlin/com/elon/app/esk/handoff/EskSnapshotConsentView.kt",
            "main/kotlin/com/elon/app/esk/handoff/EskSnapshotHttpsReader.kt",
            "main/kotlin/com/elon/app/esk/handoff/EskSnapshotRequest.kt",
            "main/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotCaller.kt",
            "main/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotConsentActivity.kt",
            "main/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotConsentView.kt",
            "main/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotWire.kt",
            "main/kotlin/com/elon/eskcontract/EskSnapshotContract.kt",
            "main/kotlin/com/elon/eskcontract/EskPlatformSnapshotContract.kt",
            "main/res/layout/esk_platform_consent_preview.xml",
            "test/kotlin/com/elon/eskcontract/EskSnapshotContractTest.kt",
            "test/kotlin/com/elon/eskcontract/EskPlatformSnapshotContractTest.kt",
            "test/kotlin/com/elon/app/esk/handoff/EskSnapshotProviderTest.kt",
            "test/kotlin/com/elon/app/esk/handoff/EskSnapshotHttpsReaderTest.kt",
            "test/kotlin/com/elon/app/esk/platform/handoff/EskPlatformProviderWiringTest.kt",
            "test/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotCallerTest.kt",
            "test/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotWireTest.kt",
        )
        old.forEach { assertFalse("Retired native artifact remains: $it", Files.exists(root().resolve("android/app/src/$it"))) }
        assertFalse(Files.exists(root().resolve("scripts/test-esk-native-snapshot-contract.js")))
    }

    @Test fun manifestHasOnlyNewCrossApkProviderWhileLocalBusinessActivitiesRemainPrivate() {
        val manifest = read("android/app/src/main/AndroidManifest.xml")
        val doc = DocumentBuilderFactory.newInstance().apply {
            isNamespaceAware = true
            setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
        }.newDocumentBuilder().parse(ByteArrayInputStream(manifest.toByteArray(Charsets.UTF_8)))
        for (old in listOf("EskSnapshotConsentActivity", "EskPlatformSnapshotConsentActivity",
            "READ_ESK_SNAPSHOT", "READ_ESK_PLATFORM_SNAPSHOT")) assertFalse(manifest.contains(old))
        val nodes = doc.getElementsByTagName("activity")
        val activities = (0 until nodes.length).map { nodes.item(it) as Element }
        val android = "http://schemas.android.com/apk/res/android"
        for (name in listOf(".esk.platform.EskPlatformAssetsActivity", ".esk.platform.EskPlatformHistoryActivity",
            ".esk.platform.sellback.EskPlatformSellbackActivity")) {
            val activity = activities.single { it.getAttributeNS(android, "name") == name }
            assertEquals("false", activity.getAttributeNS(android, "exported"))
        }
        val provider = activities.single { it.getAttributeNS(android, "name") ==
            ".esk.platform.progress.EskPlatformProgressConsentActivity" }
        assertEquals("true", provider.getAttributeNS(android, "exported"))
        assertEquals(0, provider.getElementsByTagName("intent-filter").length)
    }

    @Test fun personalProfileLedgerHistoryAndSellbackActionsAreNotCompatibilityShims() {
        val profile = kotlin("MainProfileQuickActions.kt")
        assertTrue(profile.contains("eskAssetCard.attachAndRefresh()"))
        assertTrue(profile.contains("EskPlatformProfileEntry.attach(activity, binding)"))
        assertTrue(kotlin("esk/platform/EskPlatformProfileEntry.kt").contains("EskPlatformAssetsActivity::class.java"))
        val activity = kotlin("esk/platform/EskPlatformAssetsActivity.kt")
        for (name in listOf("EskPlatformAccountReader", "EskPlatformSessionStore", "EskPlatformRequestGate",
            "EskPlatformHistoryActivity::class.java", "EskPlatformSellbackActivity::class.java")) assertTrue(activity.contains(name))
        assertFalse(activity.contains("setResult("))
        val view = kotlin("esk/platform/EskPlatformAssetsView.kt")
        assertTrue(view.contains("account.entryCount")) // Registration count was not renamed to sellback count.
        assertTrue(view.contains("account.entries.forEach"))
        for (file in listOf("EskAssetApi.kt", "EskAssetCard.kt", "EskSellbackDialog.kt", "EskPaperExchangeDialog.kt"))
            assertTrue(Files.isRegularFile(root().resolve("android/app/src/main/kotlin/com/elon/app/esk/$file")))
    }

    @Test fun formalAndPaperBackendRoutesAndLedgerSourcesStillExist() {
        val formal = read("server/src/esk_platform/mod.rs")
        for (marker in listOf("/api/me/assets/esk/platform", "/api/me/assets/esk/platform/history",
            "post(api::prepare_allocation)", "post(api::record_allocation)", "post(api::cancel_allocation)",
            "middleware::from_fn(api::private_no_store)")) assertTrue(formal.contains(marker))
        val sellback = read("server/src/esk_platform/sellback/mod.rs")
        for (marker in listOf("get(api::list).post(api::submit)", "post(api::lookup)", "post(api::cancel)"))
            assertTrue(sellback.contains(marker))
        val paper = read("server/src/esk_asset/mod.rs")
        for (marker in listOf("/api/me/assets/esk", "post(api::create_my_sellback_request)",
            "post(api::cancel_my_sellback_request)", "post(quant_allocation_api::apply_my_receipt)")) assertTrue(paper.contains(marker))
        for (file in listOf("server/src/esk_asset/model.rs", "server/src/esk_asset/service.rs", "server/src/esk_exchange/mod.rs",
            "server/src/esk_platform/model.rs", "server/src/esk_platform/sellback/migration.rs",
            "server/src/store/common/esk_platform_assets/sellback/write.rs",
            "server/src/store/common/esk_platform_assets/sellback/cancel.rs")) assertTrue(Files.isRegularFile(root().resolve(file)))
    }

    private fun kotlin(relative: String) = read("android/app/src/main/kotlin/com/elon/app/$relative")
    private fun read(relative: String) = String(Files.readAllBytes(root().resolve(relative)), Charsets.UTF_8)
    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()) { it.parent }
        .take(6).first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
}
