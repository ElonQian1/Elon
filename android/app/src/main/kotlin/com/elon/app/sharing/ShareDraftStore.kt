package com.elon.app.sharing

import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.net.Uri
import com.elon.app.PendingAttachment
import com.elon.app.displayNameForUri
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID

internal data class ShareDraft(val id: String, var text: String, val files: List<PendingAttachment>, var state: String = "ready", var owner: String = "", var target: String = "")

internal class ShareDraftStore(private val context: Context) {
    private val root = File(context.cacheDir, "external_shares").apply { mkdirs() }
    fun import(intent: Intent): ShareDraft {
        removeExpired()
        require(intent.action in setOf(Intent.ACTION_SEND, Intent.ACTION_SEND_MULTIPLE)) { "不支持的分享方式" }
        val text = SharedIntentText.read(intent)
        require(text.length <= 20_000) { "分享文字过长，请缩短后重试" }
        @Suppress("DEPRECATION")
        val streams = if (intent.action == Intent.ACTION_SEND_MULTIPLE) intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM).orEmpty()
            else listOfNotNull(intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM))
        val clip = intent.clipData
        val uris = (streams + (0 until (clip?.itemCount ?: 0)).mapNotNull { clip?.getItemAt(it)?.uri }).distinct()
        require(uris.size <= 6) { "每次最多分享 6 个附件" }
        require(text.isNotBlank() || uris.isNotEmpty()) { "分享内容为空，请尝试复制链接或从浏览器分享" }
        val id = UUID.randomUUID().toString()
        val dir = File(root, id).apply { mkdirs() }
        try {
            val files = uris.mapIndexed { index, uri ->
                require(uri.scheme == "content") { "分享附件地址不受支持，请重新选择文件" }
                val type = context.contentResolver.getType(uri).orEmpty().ifBlank { intent.type.orEmpty() }
                require(type.startsWith("image/") || type.startsWith("video/") || type == "application/pdf") { "支持图片、视频和 PDF；其他内容请复制链接" }
                val name = (displayNameForUri(context, uri) ?: "分享附件").take(180)
                val file = File(dir, "media_$index")
                context.contentResolver.openInputStream(uri).use { input ->
                    requireNotNull(input) { "无法读取分享附件，请从原应用重新分享" }
                    file.outputStream().use { output ->
                        val buffer = ByteArray(16 * 1024); var total = 0
                        while (true) { val read = input.read(buffer); if (read < 0) break; total += read
                            require(total <= 8 * 1024 * 1024) { "单个附件不能超过 8 MB" }; output.write(buffer, 0, read) }
                        require(total > 0) { "分享附件为空" }
                    }
                }
                val image = type.startsWith("image/")
                val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                if (image) { BitmapFactory.decodeFile(file.path, bounds); require(bounds.outWidth > 0 && bounds.outHeight > 0) { "图片无法解码，请重新分享原图" } }
                PendingAttachment(if (image) "image" else "file", "分享附件", name, name, type, file,
                    if (image) bounds.outWidth else null, if (image) bounds.outHeight else null)
            }
            return ShareDraft(id, text, files).also(::save)
        } catch (error: Exception) { dir.listFiles()?.forEach { it.delete() }; dir.delete(); throw error }
    }
    @Synchronized fun save(draft: ShareDraft) {
        val dir = directory(draft.id) ?: return
        val files = JSONArray(); draft.files.forEach { a -> files.put(JSONObject().put("file", a.file.name).put("name", a.displayName).put("mime", a.mimeType).put("kind", a.kind).put("width", a.imageWidth).put("height", a.imageHeight).put("source_link", a.sourceLink?.json())) }
        val json = JSONObject().put("text", draft.text).put("state", draft.state).put("owner", draft.owner).put("target", draft.target).put("files", files)
        val atomic = android.util.AtomicFile(File(dir, "draft.json"))
        var output: java.io.FileOutputStream? = null
        try { output = atomic.startWrite(); output.write(json.toString().toByteArray(Charsets.UTF_8)); atomic.finishWrite(output) }
        catch (error: Exception) { atomic.failWrite(output); throw error }
    }
    fun read(id: String): ShareDraft? = runCatching {
        val dir = directory(id) ?: return null
        val json = JSONObject(android.util.AtomicFile(File(dir, "draft.json")).openRead().bufferedReader(Charsets.UTF_8).use { it.readText() })
        val array = json.getJSONArray("files")
        val files = (0 until array.length()).map { index ->
            val a = array.getJSONObject(index); val file = File(dir, a.getString("file"))
            require(file.canonicalFile.parentFile == dir.canonicalFile && file.isFile)
            PendingAttachment(a.getString("kind"), "分享附件", a.getString("name"), a.getString("name"), a.getString("mime"), file,
                a.optInt("width").takeIf { it > 0 }, a.optInt("height").takeIf { it > 0 }, sourceLink = SourceLink.fromJson(a.optJSONObject("source_link")))
        }
        ShareDraft(id, json.getString("text"), files, json.optString("state", "ready"), json.optString("owner"), json.optString("target"))
    }.getOrNull()
    fun remove(draft: ShareDraft) { directory(draft.id)?.let { dir -> dir.listFiles()?.forEach { it.delete() }; dir.delete() } }
    private fun removeExpired() {
        val cutoff = System.currentTimeMillis() - 7 * 24 * 60 * 60 * 1000L
        root.listFiles()?.filter { it.isDirectory && it.name.matches(Regex("[a-f0-9-]{36}")) && it.lastModified() < cutoff }
            ?.forEach { dir -> if (dir.canonicalFile.parentFile == root.canonicalFile) { dir.listFiles()?.filter { it.isFile && it.canonicalFile.parentFile == dir.canonicalFile }?.forEach { it.delete() }; dir.delete() } }
    }
    private fun directory(id: String): File? = id.takeIf { it.matches(Regex("[a-f0-9-]{36}")) }?.let { File(root, it) }?.takeIf { it.isDirectory }
}
