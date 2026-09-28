package com.elon.app.chatrecords

import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import android.app.Application

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordCacheTest {
    @get:Rule val temporary = TemporaryFolder()
    @Test fun reopeningRevalidatesButDoesNotDownloadUnchangedBody() {
        val root = temporary.newFolder()
        val first = ChatRecordCache(root, "server/account/session").read("record") {
            assertNull(it); ChatRecordCache.Download(200, "\"v1\"", "body".toByteArray())
        }
        val second = ChatRecordCache(root, "server/account/session").read("record") {
            assertEquals("\"v1\"", it); ChatRecordCache.Download(304, null, byteArrayOf())
        }
        assertEquals(first, second); assertEquals("body", second.readText())
    }
    @Test fun accountScopeNeverReusesAnotherAccountsBytes() {
        val root = temporary.newFolder()
        ChatRecordCache(root, "one").read("record") { ChatRecordCache.Download(200, "v1", byteArrayOf(1)) }
        ChatRecordCache(root, "two").read("record") { assertNull(it); ChatRecordCache.Download(200, "v2", byteArrayOf(2)) }
    }
    @Test fun revokedAccessDeletesCachedBytesAndNeverServesStale() {
        val root = temporary.newFolder(); val cache = ChatRecordCache(root, "one")
        val file = cache.read("record") { ChatRecordCache.Download(200, "v1", byteArrayOf(1)) }
        assertTrue(runCatching { cache.read("record") { ChatRecordCache.Download(403, null, byteArrayOf()) } }.isFailure)
        assertFalse(file.exists())
    }
    @Test fun corruptBytesAreFetchedAgainEvenWhenLengthIsUnchanged() {
        val cache = ChatRecordCache(temporary.newFolder(), "one")
        val file = cache.read("record") { ChatRecordCache.Download(200, "v1", byteArrayOf(1)) }
        file.writeBytes(byteArrayOf(2))
        val result = cache.read("record") { assertNull(it); ChatRecordCache.Download(200, "v1", byteArrayOf(1)) }
        assertArrayEquals(byteArrayOf(1), result.readBytes())
    }
}
