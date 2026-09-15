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
}
