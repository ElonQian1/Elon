package com.elon.app.chatgptweb

import android.content.Context
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Handler
import android.os.Looper
import androidx.core.content.FileProvider
import com.elon.app.PendingAttachment
import okhttp3.Call
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import java.io.File
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/** Downloads only server-selected platform files. No login headers cross to a file URL. */
internal class GroupWebAiAttachments(
    private val context: Context,
    private val http: OkHttpClient,
    private val server: String,
    private val manifest: JSONArray,
    private val ownerCurrent: () -> Boolean,
) : AutoCloseable {
    private val handler = Handler(Looper.getMainLooper())
    private val directory = File(context.cacheDir, "chatgpt_web_uploads/group_${UUID.randomUUID()}")
    @Volatile private var closed = false
    @Volatile private var call: Call? = null
    val hasFiles get() = manifest.length() > 0
    var files: List<PendingAttachment> = emptyList()
        private set
    val uris: List<Uri> get() = files.map { FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", it.file) }

    fun load(done: (Boolean) -> Unit) {
        thread(name = "group-ai-attachments") {
            val result = runCatching {
                check(manifest.length() in 1..9)
                check(directory.mkdirs())
                // The platform download route is already readable by its opaque attachment URL.
                val client = http.newBuilder().followRedirects(false).followSslRedirects(false)
                    .callTimeout(45, TimeUnit.SECONDS).build()
                (0 until manifest.length()).map { index ->
                    check(!closed && ownerCurrent())
                    val item = manifest.getJSONObject(index)
                    val path = item.getString("download_path")
                    check(Regex("^/api/user/[^/]+/chat-attachments/[^/]+/[^/?#]+$").matches(path))
                    check(listOf("%2f", "%5c", "%00", "%25").none { path.contains(it, true) })
                    val expected = item.getLong("size_bytes")
                    check(expected in 1..8L * 1024 * 1024)
                    val target = File(directory, "file_$index")
                    val digest = MessageDigest.getInstance("SHA-256")
                    val download = client.newCall(Request.Builder().url(server.trimEnd('/') + path).get().build())
                    call = download
                    download.execute().use { response ->
                        check(response.isSuccessful)
                        val body = requireNotNull(response.body)
                        check(body.contentLength() == -1L || body.contentLength() == expected)
                        body.byteStream().use { input -> target.outputStream().use { output ->
                            val buffer = ByteArray(64 * 1024)
                            var total = 0L
                            while (true) {
                                check(!closed && ownerCurrent())
                                val count = input.read(buffer)
                                if (count < 0) break
                                total += count
                                check(total <= expected)
                                digest.update(buffer, 0, count)
                                output.write(buffer, 0, count)
                            }
                            check(total == expected)
                        } }
                    }
                    val hash = item.optString("sha256").takeIf { it.matches(Regex("[a-fA-F0-9]{64}")) }
                    check(hash == null || hash.equals(digest.digest().joinToString("") { "%02x".format(it) }, true))
                    val mime = item.getString("mime_type")
                    val bounds = if (mime.startsWith("image/")) BitmapFactory.Options().apply {
                        inJustDecodeBounds = true
                        BitmapFactory.decodeFile(target.path, this)
                    } else null
                    PendingAttachment(kind = if (bounds != null) "image" else "file", displayName = item.getString("name"),
                        fileName = item.getString("name"), mimeType = mime, file = target,
                        imageWidth = bounds?.outWidth, imageHeight = bounds?.outHeight, chatGptUploadCopy = true)
                }
            }
            call = null
            handler.post {
                if (closed || !ownerCurrent()) { close(); return@post }
                files = result.getOrDefault(emptyList())
                done(result.isSuccess)
            }
            if (closed) directory.deleteRecursively()
        }
    }

    override fun close() {
        closed = true
        call?.cancel()
        directory.deleteRecursively()
    }
}
