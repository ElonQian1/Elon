package com.elon.app.update

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdateDeliveryContractTest {
    @Test
    fun realtimeUpdateIsReplayedAndHandledAtApplicationScope() {
        val globalWs = source("android/app/src/main/kotlin/com/elon/app/GlobalWsManager.kt")
        val application = source("android/app/src/main/kotlin/com/elon/app/ElonApplication.kt")

        assertTrue(globalWs.contains("latestAppUpdate"))
        assertTrue(globalWs.contains("listeners.addIfAbsent(listener)"))
        assertTrue(globalWs.contains("listener.onGlobalWsEvent(event)"))
        assertTrue(application.contains("UpdateCheckWorker.enqueueImmediate"))
        assertTrue(application.contains("is GlobalWsEvent.AppUpdateAvailable"))
    }

    @Test
    fun downloadUsesForegroundWorkResumeAndIntegrityVerification() {
        val worker = source(
            "android/app/src/main/kotlin/com/elon/app/update/AppUpdateDownloadWorker.kt"
        )
        assertTrue(worker.contains("CoroutineWorker"))
        assertTrue(worker.contains("setForeground("))
        assertTrue(worker.contains("header(\"Range\""))
        assertTrue(worker.contains("MessageDigest.getInstance(\"SHA-256\")"))
        assertTrue(worker.contains("PART_FILE_NAME"))
        assertTrue(worker.contains("ExistingWorkPolicy.REPLACE"))
    }

    @Test
    fun activityFacadeNoLongerOwnsRawDownloadThreadOrGenericProgressDialog() {
        val manager = source("android/app/src/main/kotlin/com/elon/app/update/AppUpdateManager.kt")
        val layout = source("android/app/src/main/res/layout/sheet_app_update.xml")
        val sheetBackground = source("android/app/src/main/res/drawable/bg_update_sheet.xml")
        val primaryBackground = source("android/app/src/main/res/drawable/bg_update_primary.xml")
        val web = source("server/src/assets/web_page.html")

        assertFalse(manager.contains("Thread {"))
        assertFalse(manager.contains("progressBarStyleHorizontal"))
        assertTrue(manager.contains("AppUpdateSheet("))
        assertTrue(layout.contains("@drawable/bg_update_sheet"))
        assertTrue(sheetBackground.contains("@color/elon_surface_card"))
        assertTrue(primaryBackground.contains("@color/elon_button_primary_bg"))
        assertTrue(layout.contains("android:text=\"后台下载\"").not())
        assertTrue(web.contains("网页版自动更新"))
        assertTrue(web.contains("APK 安装包"))
    }

    private fun source(relativePath: String): String {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        val path: Path = generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .firstOrNull(Files::isRegularFile)
            ?: error("Unable to find $relativePath from $cwd")
        return String(Files.readAllBytes(path), StandardCharsets.UTF_8)
    }
}
