package com.elon.app.sharing

import android.content.Intent
import org.robolectric.RuntimeEnvironment
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], application = android.app.Application::class, manifest = Config.NONE)
class ShareDraftStoreTest {
    @Test fun retainsDraftAndUncertainStateAcrossProcessRecreation() {
        val context = RuntimeEnvironment.getApplication()
        val store = ShareDraftStore(context)
        val draft = store.import(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, "https://example.com/article"))
        draft.state = "sending"; draft.owner = "test-user"; draft.target = "测试群"; store.save(draft)
        val restored = ShareDraftStore(context).read(draft.id)!!
        assertEquals(draft.text, restored.text); assertEquals("sending", restored.state); assertEquals("test-user", restored.owner)
        assertNull(store.read("../../other-private-file")); store.remove(draft); assertNull(store.read(draft.id))
    }
    @Test fun rejectsEmptyAndUnsupportedIntentWithoutSending() {
        val store = ShareDraftStore(RuntimeEnvironment.getApplication())
        assertTrue(runCatching { store.import(Intent(Intent.ACTION_VIEW).setData(android.net.Uri.parse("https://a.test"))) }.isFailure)
        assertTrue(runCatching { store.import(Intent(Intent.ACTION_SEND).setType("text/plain")) }.isFailure)
    }
    @Test fun handlesBrowserHtmlAndRetainsTextBesideAnImage() {
        val html = Intent(Intent.ACTION_SEND).setType("text/html").putExtra(Intent.EXTRA_HTML_TEXT, "<a href=\"https://example.com/article?scene=90\">文章标题</a>")
        assertEquals("https://example.com/article?scene=90", SharedIntentText.read(html))
        val both = Intent(Intent.ACTION_SEND).setType("image/png").putExtra(Intent.EXTRA_TEXT, "https://example.com/article")
        assertEquals("https://example.com/article", SharedIntentText.read(both))
    }
    @Test fun restoresRecordTreeAndStablePublicationAfterResponseLoss() {
        val store = ShareDraftStore(RuntimeEnvironment.getApplication())
        val draft = store.import(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, "fixture"))
        draft.record = com.elon.app.chatrecords.WechatTextParser.parse("·A\n2026年09月27日 12:00\nOriginal")
        draft.publishedRecord = draft.record
        draft.recordTargetId = "group_test"; draft.state = "sending"; store.save(draft)
        val restored = ShareDraftStore(RuntimeEnvironment.getApplication()).read(draft.id)!!
        assertEquals(draft.record, restored.record); assertEquals(draft.publishedRecord, restored.publishedRecord)
        assertEquals(draft.id, restored.id); assertEquals("group_test", restored.recordTargetId); assertEquals("sending", restored.state)
        store.remove(draft)
    }
    @Test fun receivesSingleZipFromSendMultipleAndClipDataOnly() {
        val context = RuntimeEnvironment.getApplication()
        val bytes = java.io.ByteArrayOutputStream().also { buffer ->
            java.util.zip.ZipOutputStream(buffer).use { zip ->
                zip.putNextEntry(java.util.zip.ZipEntry("聊天记录.txt"))
                zip.write("·A\n2026年09月27日 12:00\nRecord fixture".toByteArray(Charsets.UTF_8)); zip.closeEntry()
            }
        }.toByteArray()
        val uri = android.net.Uri.parse("content://fixture/records.zip")
        for (clipOnly in listOf(false, true)) {
            org.robolectric.Shadows.shadowOf(context.contentResolver).registerInputStream(uri, bytes.inputStream())
            val intent = Intent(Intent.ACTION_SEND_MULTIPLE).setType("application/zip")
            if (clipOnly) intent.clipData = android.content.ClipData.newRawUri("record", uri)
            else intent.putParcelableArrayListExtra(Intent.EXTRA_STREAM, arrayListOf(uri))
            val store = ShareDraftStore(context); val draft = store.import(intent)
            assertEquals(1, draft.record!!.messages.size); assertEquals("ready", draft.state)
            assertTrue(draft.files.isEmpty()); assertEquals(draft.record, store.read(draft.id)!!.record)
            store.remove(draft)
        }
    }
}
