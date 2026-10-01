package com.elon.app.scan

import java.nio.file.Files
import java.nio.file.Paths
import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ScanPayloadTest {
    @Test fun sharedPayloadFixturesMatchNativeClassification() {
        var root = Paths.get("").toAbsolutePath()
        while (!Files.exists(root.resolve("shared/scan/payload-fixtures.json"))) root = root.parent ?: error("Fixture root missing")
        val fixtures = JSONArray(String(Files.readAllBytes(root.resolve("shared/scan/payload-fixtures.json")), Charsets.UTF_8))
        for (index in 0 until fixtures.length()) {
            val fixture = fixtures.getJSONObject(index)
            val result = ScanPayloadParser.parse(fixture.getString("raw"))
            assertEquals("fixture $index", fixture.getString("kind"), result.kind)
            if (fixture.has("target")) assertEquals(fixture.getString("target"), result.target)
            if (result.kind in listOf("text", "wifi", "contact")) assertEquals("", result.target)
        }
    }
    @Test fun rejectsOversizedOrEmptyPayloads() {
        for (raw in listOf("", "x".repeat(8193))) assertTrue(runCatching { ScanPayloadParser.parse(raw) }.isFailure)
        assertTrue(runCatching { ScanPayloadParser.friendQr("invalid") }.isFailure)
    }
}
