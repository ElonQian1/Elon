package com.elon.app.update

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdateModelsTest {
    @Test
    fun parsesReleaseMetadataIncludingIntegrityAndMirrors() {
        val version = AppUpdateVersion.parse(
            """
            {
              "versionCode": 861,
              "versionName": "1.1.853",
              "downloadUrl": "https://example.test/latest.apk",
              "changelog": "可靠更新",
              "forceUpdate": false,
              "fileSize": 8186809,
              "sha256": "ABCDEF",
              "mirrors": [
                {"url":"http://192.168.1.8/app.apk","type":"peer","priority":20}
              ]
            }
            """.trimIndent()
        )

        assertNotNull(version)
        assertEquals(861, version?.versionCode)
        assertEquals("abcdef", version?.sha256)
        assertEquals(8186809L, version?.fileSize)
        assertEquals("同 WiFi 设备", version?.downloadSources()?.first()?.displayName)
        assertEquals("官方服务器", version?.downloadSources()?.last()?.displayName)
    }

    @Test
    fun rejectsMetadataWithoutInstallableUrl() {
        assertNull(AppUpdateVersion.parse("""{"versionCode":861,"versionName":"1.1.853"}"""))
    }

    @Test
    fun snapshotRoundTripKeepsBackgroundProgress() {
        val original = AppUpdateSnapshot(
            versionCode = 861,
            versionName = "1.1.853",
            phase = AppUpdatePhase.DOWNLOADING,
            downloadedBytes = 4_096L,
            totalBytes = 8_192L,
            bytesPerSecond = 1_024L,
            sourceName = "官方服务器",
        )
        val restored = AppUpdateSnapshot.parse(original.toJson())

        assertEquals(original, restored)
        assertEquals(50, restored?.progressPercent)
        assertTrue(formatUpdateEta(original).orEmpty().contains("秒"))
    }
}
