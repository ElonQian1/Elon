package com.elon.app.chatrecords

import android.app.Application
import java.io.File
import java.io.RandomAccessFile
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordCacheLifecycleTest {
    @get:Rule val temporary = TemporaryFolder()
    private fun body() = ChatRecordCache.Download(200, "v1", "fixture".toByteArray())
    @Test fun freshReopenIsLocalButRepeatedReadsDoNotExtendValidationLease() {
        val root = temporary.newFolder(); var now = System.currentTimeMillis()
        val cache = ChatRecordCache(root, "server/account/session/record") { now }
        val file = cache.read("document") { body() }
        assertEquals(file, ChatRecordCache(root, "server/account/session/record") { now }.fresh("document"))
        assertNull(ChatRecordCache(root, "another-session") { now }.fresh("document"))
        now += ChatRecordCache.FRESH_MS - 1; assertEquals(file, cache.fresh("document"))
        now++; assertNull(cache.fresh("document"))
        assertTrue(file.exists())
    }
    @Test fun notModifiedRefreshesLeaseAndClockRollbackDoesNotExtendIt() {
        var now = System.currentTimeMillis(); val cache = ChatRecordCache(temporary.newFolder(), "one") { now }
        val file = cache.read("doc") { body() }; now += ChatRecordCache.FRESH_MS
        assertNull(cache.fresh("doc"))
        assertEquals(file, cache.read("doc") { assertEquals("v1", it); ChatRecordCache.Download(304, null, byteArrayOf()) })
        assertEquals(file, cache.fresh("doc")); now--; assertNull(cache.fresh("doc"))
    }
    @Test fun stalledAssetDoesNotBlockAnotherReaderOrCachedDocument() {
        val root = temporary.newFolder(); val cache = ChatRecordCache(root, "one")
        val doc = cache.read("doc") { body() }; val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val pool = Executors.newFixedThreadPool(2)
        try {
            val stalled = pool.submit<File> { ChatRecordCache(root, "one").read("video") { entered.countDown(); check(release.await(5, TimeUnit.SECONDS)); body() } }
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            assertEquals(doc, pool.submit<File?> { cache.fresh("doc") }.get(1, TimeUnit.SECONDS))
            release.countDown(); stalled.get(2, TimeUnit.SECONDS)
        } finally { release.countDown(); pool.shutdownNow() }
    }
    @Test fun clearCannotBeUndoneByAnInflightDownloadOrLatePoster() {
        val root = temporary.newFolder(); val cache = ChatRecordCache(root, "one")
        val file = cache.read("doc") { body() }
        val result = runCatching { cache.read("video") { cache.clear(); body() } }
        assertTrue(result.isFailure); assertNull(cache.fresh("doc")); assertEquals(0, root.listFiles()!!.size)
        ChatRecordCache.storeDerived(file) { fail("Deleted source must not recreate a poster") }
    }
    @Test fun revokedRecordClearsOnlyItsRecordScopeAndNeverReturnsFreshBytes() {
        val root = temporary.newFolder(); val first = ChatRecordCache(root, "one/record1"); val second = ChatRecordCache(root, "one/record2")
        first.read("doc") { body() }; second.read("doc") { body() }
        val error = runCatching { first.read("doc") { ChatRecordCache.Download(403, null, byteArrayOf()) } }.exceptionOrNull()
        assertTrue(error is ChatRecordCache.AccessDenied); assertNull(first.fresh("doc")); assertNotNull(second.fresh("doc"))
    }
    @Test fun capacityAndAgeEvictMediaTogetherWithMetadataAndPosters() {
        val root = temporary.newFolder(); val cache = ChatRecordCache(root, "one")
        val old = cache.read("old") { body() }
        val poster = File(root, old.name + ".poster.jpg")
        RandomAccessFile(poster, "rw").use { it.setLength(ChatRecordCache.MAX_BYTES) }
        root.listFiles()!!.forEach { it.setLastModified(System.currentTimeMillis() - 10000) }
        val recent = cache.read("recent") { body() }
        assertFalse(old.exists()); assertFalse(poster.exists()); assertTrue(recent.exists())
        root.listFiles()!!.forEach { it.setLastModified(System.currentTimeMillis() - ChatRecordCache.RETAIN_MS - 1000) }
        ChatRecordCache.trim(root); assertEquals(0, root.listFiles()!!.size)
    }
}
