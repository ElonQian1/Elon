package com.elon.app.chatrecords

import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.nio.file.Files
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

class WechatImportTest {
    private val text = "·A\n2026年09月27日 17:00\n[聊天记录]\n\n    ·B\n    2026年09月27日 16:00\n    Nested\n    second line\n\n    ·B\n    2026年09月27日 16:01\n    Nested\n\n·A\n2026年09月28日 02:00\n[图片] picture.jpg\n"
    @Test fun keepsNestedOrderDuplicatesAndOriginal() {
        val doc = WechatTextParser.parse(text)
        assertEquals(2, doc.children(null).size); assertEquals(2, doc.children("m1").size)
        assertEquals("Nested\nsecond line", doc.messages[1].text)
        assertEquals(text, doc.rawText); assertEquals("picture.jpg", doc.messages.last().filename)
        assertEquals(doc, ChatRecordDocument.read(doc.json()))
    }
    @Test fun unknownFormatRemainsReadableAndHeadersInsideTextAreNotDiscarded() {
        val raw = "not a supported format\n[图片] file.jpg"
        val doc = WechatTextParser.parse(raw)
        assertEquals(raw, doc.rawText); assertEquals("unknown", doc.messages.single().kind); assertEquals(1, doc.warnings.size)
    }
    @Test fun noSilentReparentingOfMalformedIndentedRecords() {
        assertThrows(IllegalArgumentException::class.java) { WechatTextParser.parse(text.replace("[聊天记录]", "plain")) }
    }
    private fun archive(vararg entries: Pair<String, ByteArray>, action: (File, File) -> Unit) {
        val root = Files.createTempDirectory("wechat-test-").toFile()
        try {
            val zip = File(root, "input.zip")
            ZipOutputStream(zip.outputStream()).use { out -> entries.forEach { (name, bytes) -> out.putNextEntry(ZipEntry(name)); out.write(bytes); out.closeEntry() } }
            val output = File(root, "out").apply { mkdirs() }; action(zip, output)
        } finally { root.deleteRecursively() }
    }
    @Test fun mediaIsMappedToGeneratedLocalPathsNotUntrustedZipPaths() {
        archive("聊天记录.txt" to text.toByteArray(), "附件/picture.jpg" to byteArrayOf(1,2,3)) { zip, output ->
            val doc = WechatArchive.read(zip, output)
            assertEquals(1, doc.files.size); assertNotNull(doc.document.messages.last().assetId)
            assertEquals(output.canonicalFile, doc.files.values.single().canonicalFile.parentFile)
        }
    }
    @Test fun missingAndAmbiguousAssetsAreWarningsNotFabricatedMedia() {
        archive("聊天记录.txt" to text.toByteArray()) { zip, output ->
            val result = WechatArchive.read(zip, output); assertNull(result.document.messages.last().assetId); assertEquals(1, result.document.warnings.size)
        }
        archive("聊天记录.txt" to text.toByteArray(), "a/picture.jpg" to byteArrayOf(1), "b/picture.jpg" to byteArrayOf(2)) { zip, output ->
            val result = WechatArchive.read(zip, output); assertTrue(result.files.isEmpty()); assertEquals(1, result.document.warnings.size)
        }
    }
    @Test fun zipSlipAndOversizedEntriesAreRejected() {
        archive("聊天记录.txt" to text.toByteArray(), "../escape" to byteArrayOf(1)) { zip, output ->
            assertThrows(IllegalArgumentException::class.java) { WechatArchive.read(zip, output) }; assertTrue(output.listFiles()!!.isEmpty())
        }
        archive("聊天记录.txt" to text.toByteArray(), "picture.jpg" to ByteArray(12 * 1024 * 1024 + 1)) { zip, output ->
            assertThrows(IllegalArgumentException::class.java) { WechatArchive.read(zip, output) }; assertTrue(output.listFiles()!!.isEmpty())
        }
    }
    @Test fun optionalUserSamplesOnlyVerifyStructureWithoutLoggingContent() {
        val samples = System.getenv("ELON_WECHAT_TEST_SAMPLES") ?: return
        samples.split(';').filter { it.isNotBlank() }.forEachIndexed { index, path ->
            val output = Files.createTempDirectory("wechat-sample-").toFile()
            try {
                val result = WechatArchive.read(File(path), output)
                assertEquals(if (index == 0) 3 else 9, result.document.children(null).size)
                if (index == 1) { assertEquals(63, result.document.messages.size); assertEquals(2, result.files.size); assertTrue(result.document.warnings.isEmpty()) }
                println("WECHAT_SAMPLE_VERIFIED index=$index outer=${result.document.children(null).size} total=${result.document.messages.size} assets=${result.files.size}")
            } finally { output.deleteRecursively() }
        }
    }
}
