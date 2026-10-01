package com.elon.app

import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okio.Buffer
import java.io.File
import java.io.RandomAccessFile
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.After
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ChatImageDiskCacheTest {
    private val context get() = RuntimeEnvironment.getApplication()
    private lateinit var server: MockWebServer
    private val bytes = ByteArray(4096) { (it % 251).toByte() }
    private val source get() = server.url("/image").toString()

    @Before fun start() {
        ChatImageDiskCache.clearUnused(context)
        server = MockWebServer()
        repeat(8) { server.enqueue(MockResponse().setBody(Buffer().write(bytes))) }
        server.start()
    }
    @After fun stop() { server.shutdown(); ChatImageDiskCache.clearUnused(context) }

    @Test fun thumbnailAndViewerShareOneOriginalDownloadAndWorkOffline() {
        assertArrayEquals(bytes, ChatImageDiskCache.readBytes(context, source, 10000))
        val url = source
        server.shutdown()
        ChatImageDiskCache.acquire(context, url).use { assertArrayEquals(bytes, it.file.readBytes()) }
        assertEquals(1, server.requestCount)
    }

    @Test fun concurrentLoadsCoalesceAndClearCannotDeleteOpenViewer() {
        val executor = Executors.newFixedThreadPool(4)
        try {
            val jobs = (1..4).map { executor.submit<ChatImageDiskCache.Lease> { ChatImageDiskCache.acquire(context, source) } }
            val leases = jobs.map { it.get(10, TimeUnit.SECONDS) }
            try {
                assertEquals(1, server.requestCount)
                assertEquals(1, leases.map { it.file.path }.distinct().size)
                assertEquals(0L, ChatImageDiskCache.clearUnused(context))
                assertTrue(leases.all { it.file.exists() })
            } finally { leases.forEach { it.close(); it.close() } }
            assertEquals(bytes.size.toLong(), ChatImageDiskCache.clearUnused(context))
        } finally { executor.shutdownNow() }
    }

    @Test fun sizeLimitAndNetworkFailureDoNotLeavePartialCache() {
        assertThrows(IllegalArgumentException::class.java) { ChatImageDiskCache.acquire(context, source, 100) }
        assertEquals(0L, ChatImageDiskCache.sizeBytes(context))
        assertFalse(File(context.cacheDir, "chat_image_cache").listFiles().orEmpty().any { it.extension == "part" })
        ChatImageDiskCache.acquire(context, source).use { assertArrayEquals(bytes, it.file.readBytes()) }
    }

    @Test fun evictionIsBoundedAndNeverDeletesLocalAttachment() {
        val lease = ChatImageDiskCache.acquire(context, source)
        val dir = lease.file.parentFile!!
        repeat(8) { index ->
            val file = File(dir, "old-$index.img")
            RandomAccessFile(file, "rw").use { it.setLength(12L * 1024 * 1024) }
            file.setLastModified(index + 1L)
        }
        lease.close()
        assertTrue(ChatImageDiskCache.sizeBytes(context) <= 64L * 1024 * 1024)
        assertTrue(lease.file.exists())
        val local = File.createTempFile("local", ".png", context.cacheDir).apply { writeBytes(bytes) }
        ChatImageDiskCache.acquire(context, local.path).use { ChatImageDiskCache.clearUnused(context) }
        assertTrue(local.exists())
    }

    @Test fun smallerCallerLimitDoesNotEvictAnExistingOriginal() {
        ChatImageDiskCache.acquire(context, source).close()
        assertThrows(IllegalArgumentException::class.java) { ChatImageDiskCache.acquire(context, source, 100) }
        ChatImageDiskCache.acquire(context, source).use { assertArrayEquals(bytes, it.file.readBytes()) }
        assertEquals(1, server.requestCount)
    }
}
